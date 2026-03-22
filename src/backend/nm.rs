use std::collections::{BTreeMap, HashSet};
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use anyhow::{Context, anyhow};
use nmrs::builders::WifiConnectionBuilder;
use nmrs::{ConnectionError, NetworkManager, WifiSecurity};
use tokio::runtime::Runtime;
use tracing::error;
use zbus::Connection;
use zvariant::{OwnedObjectPath, OwnedValue, Value};

use crate::model::{
    AccessPointGroup, AppSnapshot, ConnectRequest, CurrentConnection, SecurityKind, WifiCommand,
    WifiEvent, band_label, group_networks,
};

pub type CommandSender = Sender<WifiCommand>;
const FALLBACK_REFRESH_INTERVAL: Duration = Duration::from_secs(5);

pub trait WifiController: Send + Sync {
    fn send(&self, command: WifiCommand) -> anyhow::Result<()>;
    fn try_recv(&self) -> Option<WifiEvent>;
}

pub struct BackendController {
    command_tx: Sender<WifiCommand>,
    event_rx: Mutex<Receiver<WifiEvent>>,
}

struct BackendWorker {
    manager: NetworkManager,
    connection: Connection,
    event_tx: Sender<WifiEvent>,
    transient_error: Option<String>,
    monitor_cancellers: Vec<tokio::sync::oneshot::Sender<()>>,
}

enum BackendSignal {
    Command(WifiCommand),
    SnapshotRefresh,
    MonitorFailed(&'static str, String),
}

impl BackendController {
    pub fn spawn() -> anyhow::Result<Self> {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();

        thread::Builder::new()
            .name("innu-networkmanager".into())
            .spawn(move || {
                if let Err(error) = backend_thread(command_rx, event_tx) {
                    error!(?error, "backend thread exited");
                }
            })
            .context("failed to spawn backend thread")?;

        Ok(Self {
            command_tx,
            event_rx: Mutex::new(event_rx),
        })
    }

    pub fn command_sender(&self) -> Sender<WifiCommand> {
        self.command_tx.clone()
    }
}

impl WifiController for BackendController {
    fn send(&self, command: WifiCommand) -> anyhow::Result<()> {
        self.command_tx
            .send(command)
            .map_err(|_| anyhow!("backend worker is not available"))
    }

    fn try_recv(&self) -> Option<WifiEvent> {
        self.event_rx
            .lock()
            .expect("event rx mutex poisoned")
            .try_recv()
            .ok()
    }
}

fn backend_thread(
    command_rx: Receiver<WifiCommand>,
    event_tx: Sender<WifiEvent>,
) -> anyhow::Result<()> {
    let runtime = Runtime::new().context("failed to create tokio runtime")?;
    let (signal_tx, signal_rx) = mpsc::channel();
    let relay_tx = signal_tx.clone();
    thread::Builder::new()
        .name("innu-networkmanager-relay".into())
        .spawn(move || {
            while let Ok(command) = command_rx.recv() {
                if relay_tx.send(BackendSignal::Command(command)).is_err() {
                    break;
                }
            }
        })
        .context("failed to spawn backend relay thread")?;

    let mut worker = initialize_worker(&runtime, &event_tx, &signal_tx);

    loop {
        match signal_rx.recv_timeout(FALLBACK_REFRESH_INTERVAL) {
            Ok(BackendSignal::Command(WifiCommand::Shutdown)) => break,
            Ok(BackendSignal::Command(command)) => {
                if worker.is_none() {
                    worker = initialize_worker(&runtime, &event_tx, &signal_tx);
                }

                if let Some(active_worker) = worker.as_mut() {
                    if let Err(error) = runtime.block_on(active_worker.handle(command)) {
                        let message = active_worker
                            .transient_error
                            .clone()
                            .unwrap_or_else(|| error.to_string());
                        let _ = active_worker
                            .event_tx
                            .send(WifiEvent::ErrorRaised(message.clone()));
                        match runtime.block_on(active_worker.snapshot()) {
                            Ok(snapshot) => {
                                let _ = active_worker
                                    .event_tx
                                    .send(WifiEvent::SnapshotUpdated(Box::new(snapshot)));
                            }
                            Err(snapshot_error) => {
                                let _ = active_worker.event_tx.send(WifiEvent::SnapshotUpdated(
                                    Box::new(unavailable_snapshot(
                                        true,
                                        Some(snapshot_error.to_string()),
                                    )),
                                ));
                                worker = None;
                            }
                        }
                        continue;
                    }

                    match runtime.block_on(active_worker.snapshot()) {
                        Ok(snapshot) => {
                            let _ = active_worker
                                .event_tx
                                .send(WifiEvent::SnapshotUpdated(Box::new(snapshot)));
                        }
                        Err(error) => {
                            let message = error.to_string();
                            let _ = active_worker
                                .event_tx
                                .send(WifiEvent::ErrorRaised(message.clone()));
                            let _ =
                                active_worker
                                    .event_tx
                                    .send(WifiEvent::SnapshotUpdated(Box::new(
                                        unavailable_snapshot(true, Some(message)),
                                    )));
                            worker = None;
                        }
                    }
                } else {
                    let _ = event_tx.send(WifiEvent::ErrorRaised(
                        "NetworkManager is unavailable.".into(),
                    ));
                }
            }
            Ok(BackendSignal::SnapshotRefresh) | Err(RecvTimeoutError::Timeout) => {
                if worker.is_none() {
                    worker = initialize_worker(&runtime, &event_tx, &signal_tx);
                }

                if let Some(active_worker) = worker.as_mut() {
                    match runtime.block_on(active_worker.snapshot()) {
                        Ok(snapshot) => {
                            let _ = active_worker
                                .event_tx
                                .send(WifiEvent::SnapshotUpdated(Box::new(snapshot)));
                        }
                        Err(error) => {
                            let message = error.to_string();
                            let _ =
                                active_worker
                                    .event_tx
                                    .send(WifiEvent::SnapshotUpdated(Box::new(
                                        unavailable_snapshot(true, Some(message)),
                                    )));
                            worker = None;
                        }
                    }
                }
            }
            Ok(BackendSignal::MonitorFailed(kind, message)) => {
                let error_message = format!("{kind} monitoring stopped: {message}");
                let _ = event_tx.send(WifiEvent::ErrorRaised(error_message.clone()));
                let _ = event_tx.send(WifiEvent::SnapshotUpdated(Box::new(
                    unavailable_snapshot(true, Some(error_message)),
                )));
                worker = None;
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    Ok(())
}

fn initialize_worker(
    runtime: &Runtime,
    event_tx: &Sender<WifiEvent>,
    signal_tx: &Sender<BackendSignal>,
) -> Option<BackendWorker> {
    match runtime.block_on(BackendWorker::new(event_tx.clone())) {
        Ok(mut worker) => match runtime.block_on(worker.snapshot()) {
            Ok(snapshot) => {
                worker.start_monitors(signal_tx.clone(), snapshot.wifi_available);
                let _ = event_tx.send(WifiEvent::SnapshotUpdated(Box::new(snapshot)));
                Some(worker)
            }
            Err(error) => {
                let _ = event_tx.send(WifiEvent::SnapshotUpdated(Box::new(unavailable_snapshot(
                    true,
                    Some(error.to_string()),
                ))));
                None
            }
        },
        Err(error) => {
            let _ = event_tx.send(WifiEvent::SnapshotUpdated(Box::new(unavailable_snapshot(
                false,
                Some(error.to_string()),
            ))));
            None
        }
    }
}

impl BackendWorker {
    async fn new(event_tx: Sender<WifiEvent>) -> anyhow::Result<Self> {
        let manager = NetworkManager::new()
            .await
            .context("failed to open NetworkManager")?;
        let connection = Connection::system()
            .await
            .context("failed to open system D-Bus")?;

        Ok(Self {
            manager,
            connection,
            event_tx,
            transient_error: None,
            monitor_cancellers: Vec::new(),
        })
    }

    fn clear_error(&mut self) {
        self.transient_error = None;
    }

    fn start_monitors(&mut self, signal_tx: Sender<BackendSignal>, wifi_available: bool) {
        if wifi_available {
            let network_manager = self.manager.clone();
            let network_refresh_tx = signal_tx.clone();
            let network_error_tx = signal_tx.clone();
            let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
            if let Err(spawn_error) = thread::Builder::new()
                .name("innu-network-monitor".into())
                .spawn(move || {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build();
                    match runtime {
                        Ok(runtime) => {
                            let result = runtime.block_on(async move {
                                tokio::select! {
                                    result = network_manager.monitor_network_changes(move || {
                                        let _ = network_refresh_tx.send(BackendSignal::SnapshotRefresh);
                                    }) => result,
                                    _ = cancel_rx => Ok(()),
                                }
                            });
                            if let Err(error) = result {
                                let _ = network_error_tx.send(BackendSignal::MonitorFailed(
                                    "Network",
                                    error.to_string(),
                                ));
                            }
                        }
                        Err(error) => {
                            let _ = network_error_tx
                                .send(BackendSignal::MonitorFailed("Network", error.to_string()));
                        }
                    }
                })
            {
                error!(?spawn_error, "failed to spawn network monitor thread");
            } else {
                self.monitor_cancellers.push(cancel_tx);
            }
        }

        let device_manager = self.manager.clone();
        let device_refresh_tx = signal_tx.clone();
        let device_error_tx = signal_tx;
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
        if let Err(spawn_error) = thread::Builder::new()
            .name("innu-device-monitor".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                match runtime {
                    Ok(runtime) => {
                        let result = runtime.block_on(async move {
                            tokio::select! {
                                result = device_manager.monitor_device_changes(move || {
                                    let _ = device_refresh_tx.send(BackendSignal::SnapshotRefresh);
                                }) => result,
                                _ = cancel_rx => Ok(()),
                            }
                        });
                        if let Err(error) = result {
                            let _ = device_error_tx
                                .send(BackendSignal::MonitorFailed("Device", error.to_string()));
                        }
                    }
                    Err(error) => {
                        let _ = device_error_tx
                            .send(BackendSignal::MonitorFailed("Device", error.to_string()));
                    }
                }
            })
        {
            error!(?spawn_error, "failed to spawn device monitor thread");
        } else {
            self.monitor_cancellers.push(cancel_tx);
        }
    }

    async fn handle(&mut self, command: WifiCommand) -> anyhow::Result<()> {
        match command {
            WifiCommand::Refresh => {
                self.event_tx
                    .send(WifiEvent::OperationStarted(
                        "Refreshing nearby networks".into(),
                    ))
                    .ok();
                self.manager.scan_networks().await?;
                self.clear_error();
                self.event_tx
                    .send(WifiEvent::OperationFinished("Wi-Fi list updated".into()))
                    .ok();
                Ok(())
            }
            WifiCommand::Connect(request) => self.connect(request).await,
            WifiCommand::Disconnect => {
                self.event_tx
                    .send(WifiEvent::OperationStarted(
                        "Disconnecting current network".into(),
                    ))
                    .ok();
                self.manager.disconnect().await?;
                self.clear_error();
                self.event_tx
                    .send(WifiEvent::OperationFinished("Disconnected".into()))
                    .ok();
                Ok(())
            }
            WifiCommand::Forget(ssid) => {
                self.event_tx
                    .send(WifiEvent::OperationStarted(format!("Forgetting {ssid}")))
                    .ok();
                self.manager.forget(&ssid).await?;
                self.clear_error();
                self.event_tx
                    .send(WifiEvent::OperationFinished(format!("Forgot {ssid}")))
                    .ok();
                Ok(())
            }
            WifiCommand::Shutdown => Ok(()),
        }
    }

    async fn connect(&mut self, request: ConnectRequest) -> anyhow::Result<()> {
        self.event_tx
            .send(WifiEvent::OperationStarted(format!(
                "Connecting to {}",
                request.ssid
            )))
            .ok();

        let result = if request.hidden {
            connect_hidden(&self.connection, request.clone()).await
        } else {
            let security = match request.security {
                SecurityKind::Open => WifiSecurity::Open,
                SecurityKind::WpaPsk | SecurityKind::Unknown | SecurityKind::Enterprise => {
                    WifiSecurity::WpaPsk {
                        psk: request.passphrase.clone().unwrap_or_default(),
                    }
                }
            };
            self.manager
                .connect(&request.ssid, security)
                .await
                .map_err(anyhow::Error::from)
        };

        match result {
            Ok(()) => {
                self.clear_error();
                self.event_tx
                    .send(WifiEvent::OperationFinished(format!(
                        "Connected to {}",
                        request.ssid
                    )))
                    .ok();
                Ok(())
            }
            Err(error) => {
                self.transient_error = Some(humanize_nm_error(&error));
                Err(error)
            }
        }
    }

    async fn snapshot(&self) -> anyhow::Result<AppSnapshot> {
        let wifi_devices = self.manager.list_wireless_devices().await?;
        let primary_device_id = wifi_devices.first().map(|device| device.path.clone());
        let wifi_available = !wifi_devices.is_empty();
        let radio_enabled = self.manager.wifi_enabled().await.unwrap_or(true);
        let rfkill_blocked = !manager_bool_property(&self.connection, "WirelessHardwareEnabled")
            .await
            .unwrap_or(true);
        let saved_ids = list_saved_connection_ids(&self.connection).await?;
        let current = self.manager.current_network().await?;
        let visible = self.manager.list_networks().await?;
        let visible_networks = group_visible_networks(visible, &saved_ids, current.as_ref());
        let current_connection = current.map(|network| CurrentConnection {
            device_id: network.device,
            ssid: network.ssid,
            signal: network.strength,
            security: classify_security_flags(network.secured, network.is_psk, network.is_eap),
            band_summary: band_label(network.frequency),
        });

        Ok(AppSnapshot {
            primary_device_id,
            current_connection,
            visible_networks,
            wifi_available,
            radio_enabled,
            rfkill_blocked,
            manager_running: true,
            transient_error: self.transient_error.clone(),
        })
    }
}

impl Drop for BackendWorker {
    fn drop(&mut self) {
        for canceller in self.monitor_cancellers.drain(..) {
            let _ = canceller.send(());
        }
    }
}

fn unavailable_snapshot(manager_running: bool, error: Option<String>) -> AppSnapshot {
    AppSnapshot {
        manager_running,
        transient_error: error,
        ..Default::default()
    }
}

fn humanize_nm_error(error: &anyhow::Error) -> String {
    if let Some(connection) = error.downcast_ref::<ConnectionError>() {
        match connection {
            ConnectionError::AuthFailed => {
                "Authentication failed. Re-enter the Wi-Fi password.".into()
            }
            ConnectionError::DhcpFailed => {
                "The network accepted the credentials but did not issue an IP address.".into()
            }
            ConnectionError::NotFound => {
                "The selected network is no longer visible. Refresh and try again.".into()
            }
            ConnectionError::Timeout => "NetworkManager timed out while trying to connect.".into(),
            ConnectionError::NoSavedConnection => {
                "No saved profile exists for this network.".into()
            }
            other => other.to_string(),
        }
    } else {
        error.to_string()
    }
}

fn group_visible_networks(
    visible: Vec<nmrs::Network>,
    saved_ids: &HashSet<String>,
    current: Option<&nmrs::Network>,
) -> Vec<AccessPointGroup> {
    let mut groups: BTreeMap<String, AccessPointGroup> = BTreeMap::new();

    for network in visible {
        let entry = groups
            .entry(network.ssid.clone())
            .or_insert_with(|| AccessPointGroup {
                ssid: network.ssid.clone(),
                device_id: network.device.clone(),
                signal: network.strength,
                security: classify_security_flags(network.secured, network.is_psk, network.is_eap),
                band_summary: band_label(network.frequency),
                known: saved_ids.contains(&network.ssid),
                in_use: current
                    .map(|active| active.ssid == network.ssid)
                    .unwrap_or(false),
            });

        if network.strength.unwrap_or_default() >= entry.signal.unwrap_or_default() {
            entry.signal = network.strength;
            entry.security =
                classify_security_flags(network.secured, network.is_psk, network.is_eap);
            entry.band_summary = band_label(network.frequency);
            entry.device_id = network.device.clone();
            entry.known = saved_ids.contains(&network.ssid);
            entry.in_use = current
                .map(|active| active.ssid == network.ssid)
                .unwrap_or(false);
        }
    }

    group_networks(groups.into_values().collect())
}

fn classify_security_flags(secured: bool, is_psk: bool, is_eap: bool) -> SecurityKind {
    if !secured {
        SecurityKind::Open
    } else if is_eap {
        SecurityKind::Enterprise
    } else if is_psk {
        SecurityKind::WpaPsk
    } else {
        SecurityKind::Unknown
    }
}

async fn manager_bool_property(connection: &Connection, property: &str) -> anyhow::Result<bool> {
    let proxy = zbus::Proxy::new(
        connection,
        "org.freedesktop.NetworkManager",
        "/org/freedesktop/NetworkManager",
        "org.freedesktop.NetworkManager",
    )
    .await?;
    Ok(proxy.get_property(property).await?)
}

async fn list_saved_connection_ids(connection: &Connection) -> anyhow::Result<HashSet<String>> {
    let proxy = zbus::Proxy::new(
        connection,
        "org.freedesktop.NetworkManager",
        "/org/freedesktop/NetworkManager/Settings",
        "org.freedesktop.NetworkManager.Settings",
    )
    .await?;
    let paths: Vec<OwnedObjectPath> = proxy.call("ListConnections", &()).await?;

    let mut saved = HashSet::new();
    for path in paths {
        let connection_proxy = zbus::Proxy::new(
            connection,
            "org.freedesktop.NetworkManager",
            path.as_str(),
            "org.freedesktop.NetworkManager.Settings.Connection",
        )
        .await?;

        let settings: std::collections::HashMap<
            String,
            std::collections::HashMap<String, OwnedValue>,
        > = connection_proxy.call("GetSettings", &()).await?;
        let Some(connection_settings) = settings.get("connection") else {
            continue;
        };
        let connection_type = connection_settings
            .get("type")
            .and_then(extract_string)
            .unwrap_or_default();
        if connection_type != "802-11-wireless" {
            continue;
        }

        if let Some(id) = connection_settings.get("id").and_then(extract_string) {
            saved.insert(id);
        }
    }

    Ok(saved)
}

async fn connect_hidden(connection: &Connection, request: ConnectRequest) -> anyhow::Result<()> {
    let mut builder = WifiConnectionBuilder::new(&request.ssid)
        .hidden(true)
        .autoconnect(true)
        .ipv4_auto()
        .ipv6_auto();

    builder = match request.security {
        SecurityKind::Open => builder.open(),
        SecurityKind::WpaPsk | SecurityKind::Unknown | SecurityKind::Enterprise => {
            builder.wpa_psk(request.passphrase.unwrap_or_default())
        }
    };

    let settings = builder.build();
    let manager = zbus::Proxy::new(
        connection,
        "org.freedesktop.NetworkManager",
        "/org/freedesktop/NetworkManager",
        "org.freedesktop.NetworkManager",
    )
    .await?;
    let empty_object = OwnedObjectPath::try_from("/")?;
    let empty_options: std::collections::HashMap<&str, Value<'static>> =
        std::collections::HashMap::new();
    let _: (
        OwnedObjectPath,
        OwnedObjectPath,
        std::collections::HashMap<String, OwnedValue>,
    ) = manager
        .call(
            "AddAndActivateConnection2",
            &(
                settings,
                OwnedObjectPath::try_from(request.device_id.as_str())?,
                empty_object,
                empty_options,
            ),
        )
        .await?;
    Ok(())
}

fn extract_string(value: &OwnedValue) -> Option<String> {
    value.clone().try_into().ok()
}
