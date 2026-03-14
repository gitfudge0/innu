use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::{StandardItem, SubMenu};

use crate::backend::nm::CommandSender;
use crate::model::{signal_bars, AccessPointGroup, AppSnapshot, ConnectRequest, WifiCommand};

#[derive(Clone)]
pub struct TrayBridge {
    inner: Arc<TrayBridgeInner>,
}

struct TrayBridgeInner {
    state: Arc<Mutex<TrayState>>,
    handle: Handle<WifiTray>,
    signal_rx: Mutex<Receiver<TraySignal>>,
}

#[derive(Debug, Default)]
struct TrayState {
    title: String,
    subtitle: String,
    icon_name: String,
    connected_ssid: Option<String>,
    nearby: Vec<AccessPointGroup>,
    manager_running: bool,
    wifi_available: bool,
    radio_enabled: bool,
}

#[derive(Debug)]
struct WifiTray {
    command_tx: CommandSender,
    signal_tx: Sender<TraySignal>,
    state: Arc<Mutex<TrayState>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraySignal {
    ShowWindow,
    QuitApp,
}

impl TrayBridge {
    pub fn spawn(command_tx: CommandSender) -> anyhow::Result<Self> {
        let state = Arc::new(Mutex::new(TrayState {
            title: "Innu - Network Management".into(),
            subtitle: "Not connected".into(),
            icon_name: "network-wireless-offline".into(),
            ..Default::default()
        }));
        let (signal_tx, signal_rx) = mpsc::channel();
        let handle = WifiTray {
            command_tx,
            signal_tx,
            state: Arc::clone(&state),
        }
        .spawn()?;

        Ok(Self {
            inner: Arc::new(TrayBridgeInner {
                state,
                handle,
                signal_rx: Mutex::new(signal_rx),
            }),
        })
    }

    pub fn apply_snapshot(&self, snapshot: &AppSnapshot) {
        let mut state = self.inner.state.lock().expect("tray mutex poisoned");
        state.connected_ssid = snapshot
            .current_connection
            .as_ref()
            .map(|connection| connection.ssid.clone());
        state.title = state
            .connected_ssid
            .as_ref()
            .map(|ssid| format!("Innu - Network Management · {ssid}"))
            .unwrap_or_else(|| "Innu - Network Management".into());
        state.subtitle = if !snapshot.manager_running {
            "NetworkManager unavailable".into()
        } else if !snapshot.wifi_available {
            "No Wi-Fi adapter".into()
        } else if let Some(connection) = &snapshot.current_connection {
            format!(
                "Connected · {} {}",
                signal_bars(connection.signal),
                connection.security.label()
            )
        } else if snapshot.radio_enabled {
            "Not connected".into()
        } else {
            "Wi-Fi unavailable".into()
        };
        state.icon_name = if !snapshot.manager_running || !snapshot.wifi_available {
            "network-wireless-offline".into()
        } else if snapshot.current_connection.is_some() {
            "network-wireless".into()
        } else if snapshot.radio_enabled {
            "network-wireless-signal-excellent".into()
        } else {
            "network-wireless-offline".into()
        };
        state.nearby = snapshot.visible_networks.iter().take(6).cloned().collect();
        state.manager_running = snapshot.manager_running;
        state.wifi_available = snapshot.wifi_available;
        state.radio_enabled = snapshot.radio_enabled;
        drop(state);
        self.inner.handle.update(|_| {});
    }

    pub fn try_recv(&self) -> Option<TraySignal> {
        self.inner
            .signal_rx
            .lock()
            .expect("tray signal mutex poisoned")
            .try_recv()
            .ok()
    }
}

impl ksni::Tray for WifiTray {
    fn id(&self) -> String {
        "innu".into()
    }

    fn title(&self) -> String {
        self.state
            .lock()
            .expect("tray mutex poisoned")
            .title
            .clone()
    }

    fn icon_name(&self) -> String {
        self.state
            .lock()
            .expect("tray mutex poisoned")
            .icon_name
            .clone()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.signal_tx.send(TraySignal::ShowWindow);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let state = self.state.lock().expect("tray mutex poisoned");
        let mut items = vec![
            StandardItem {
                label: state.subtitle.clone(),
                enabled: false,
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
        ];

        if let Some(ssid) = &state.connected_ssid {
            items.push(
                StandardItem {
                    label: format!("Connected: {ssid}"),
                    enabled: false,
                    ..Default::default()
                }
                .into(),
            );
            items.push(
                StandardItem {
                    label: "Disconnect".into(),
                    icon_name: "network-disconnect".into(),
                    activate: Box::new(|tray: &mut Self| {
                        let _ = tray.command_tx.send(WifiCommand::Disconnect);
                    }),
                    ..Default::default()
                }
                .into(),
            );
            items.push(ksni::MenuItem::Separator);
        }

        if !state.nearby.is_empty() {
            let nearby = state
                .nearby
                .iter()
                .map(|network| {
                    let secure_unknown = network.security.requires_passphrase() && !network.known;
                    let unsupported = !network.security.is_supported();
                    let label = format!(
                        "{}  {}  {}{}",
                        signal_bars(network.signal),
                        network.ssid,
                        network.security.label(),
                        if network.known { " · Saved" } else { "" }
                    );
                    let request = ConnectRequest {
                        device_id: network.device_id.clone(),
                        ssid: network.ssid.clone(),
                        hidden: false,
                        security: network.security,
                        passphrase: None,
                    };
                    StandardItem {
                        label,
                        activate: Box::new(move |tray: &mut Self| {
                            if unsupported || secure_unknown {
                                let _ = tray.signal_tx.send(TraySignal::ShowWindow);
                            } else {
                                let _ = tray.command_tx.send(WifiCommand::Connect(request.clone()));
                            }
                        }),
                        ..Default::default()
                    }
                    .into()
                })
                .collect();

            items.push(
                SubMenu {
                    label: "Nearby Networks".into(),
                    submenu: nearby,
                    ..Default::default()
                }
                .into(),
            );
            items.push(
                StandardItem {
                    label: "Join Hidden Network".into(),
                    activate: Box::new(|tray: &mut Self| {
                        let _ = tray.signal_tx.send(TraySignal::ShowWindow);
                    }),
                    ..Default::default()
                }
                .into(),
            );
            items.push(ksni::MenuItem::Separator);
        }

        items.push(
            StandardItem {
                label: "Open Window".into(),
                icon_name: "preferences-system-network".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.signal_tx.send(TraySignal::ShowWindow);
                }),
                ..Default::default()
            }
            .into(),
        );
        items.push(
            StandardItem {
                label: "Refresh".into(),
                icon_name: "view-refresh".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.command_tx.send(WifiCommand::Refresh);
                }),
                ..Default::default()
            }
            .into(),
        );
        items.push(ksni::MenuItem::Separator);
        items.push(
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.signal_tx.send(TraySignal::QuitApp);
                }),
                ..Default::default()
            }
            .into(),
        );

        items
    }
}
