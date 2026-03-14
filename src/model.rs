use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Default)]
pub struct AppSnapshot {
    pub primary_device_id: Option<String>,
    pub current_connection: Option<CurrentConnection>,
    pub visible_networks: Vec<AccessPointGroup>,
    pub wifi_available: bool,
    pub radio_enabled: bool,
    pub rfkill_blocked: bool,
    pub manager_running: bool,
    pub transient_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentConnection {
    pub device_id: String,
    pub ssid: String,
    pub signal: Option<u8>,
    pub security: SecurityKind,
    pub band_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessPointGroup {
    pub ssid: String,
    pub device_id: String,
    pub signal: Option<u8>,
    pub security: SecurityKind,
    pub band_summary: String,
    pub known: bool,
    pub in_use: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectRequest {
    pub device_id: String,
    pub ssid: String,
    pub hidden: bool,
    pub security: SecurityKind,
    pub passphrase: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SecurityKind {
    Open,
    WpaPsk,
    Enterprise,
    #[default]
    Unknown,
}

impl SecurityKind {
    pub fn requires_passphrase(self) -> bool {
        matches!(self, Self::WpaPsk | Self::Unknown)
    }

    pub fn is_supported(self) -> bool {
        matches!(self, Self::Open | Self::WpaPsk | Self::Unknown)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::WpaPsk => "WPA Personal",
            Self::Enterprise => "Enterprise",
            Self::Unknown => "Secured",
        }
    }
}

#[derive(Debug, Clone)]
pub enum WifiCommand {
    Refresh,
    Connect(ConnectRequest),
    Disconnect,
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum WifiEvent {
    SnapshotUpdated(Box<AppSnapshot>),
    OperationStarted(String),
    OperationFinished(String),
    ErrorRaised(String),
}

pub fn signal_bars(signal: Option<u8>) -> &'static str {
    match signal.unwrap_or_default() {
        0..=20 => "▂",
        21..=40 => "▂▄",
        41..=60 => "▂▄▆",
        61..=80 => "▂▄▆█",
        _ => "█▆█",
    }
}

pub fn band_label(frequency: Option<u32>) -> String {
    match frequency {
        Some(freq) if freq >= 5900 => "6 GHz".into(),
        Some(freq) if freq >= 4900 => "5 GHz".into(),
        Some(_) => "2.4 GHz".into(),
        None => "Unknown band".into(),
    }
}

pub fn group_networks(mut networks: Vec<AccessPointGroup>) -> Vec<AccessPointGroup> {
    networks.sort_by(|left, right| {
        right
            .in_use
            .cmp(&left.in_use)
            .then(right.known.cmp(&left.known))
            .then(
                right
                    .signal
                    .unwrap_or_default()
                    .cmp(&left.signal.unwrap_or_default()),
            )
            .then(left.ssid.cmp(&right.ssid))
    });
    networks
}

impl Display for SecurityKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_visible_networks_by_priority() {
        let result = group_networks(vec![
            AccessPointGroup {
                ssid: "z".into(),
                device_id: "wlp0".into(),
                signal: Some(80),
                security: SecurityKind::WpaPsk,
                band_summary: "2.4 GHz".into(),
                known: false,
                in_use: false,
            },
            AccessPointGroup {
                ssid: "a".into(),
                device_id: "wlp0".into(),
                signal: Some(10),
                security: SecurityKind::Open,
                band_summary: "2.4 GHz".into(),
                known: true,
                in_use: true,
            },
        ]);

        assert_eq!(result[0].ssid, "a");
        assert!(result[0].in_use);
    }

    #[test]
    fn marks_unknown_security_as_passphrase_based() {
        assert!(SecurityKind::Unknown.requires_passphrase());
        assert!(SecurityKind::Unknown.is_supported());
        assert!(!SecurityKind::Enterprise.is_supported());
    }
}
