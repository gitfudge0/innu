use crate::model::{DiagnosticItem, DiagnosticSeverity};

pub fn notify_info(summary: &str, body: &str) {
    let _ = notify_rust::Notification::new()
        .summary(summary)
        .body(body)
        .icon("network-wireless")
        .show();
}

pub fn notify_diagnostic(item: &DiagnosticItem) {
    let urgency = match item.severity {
        DiagnosticSeverity::Info => notify_rust::Urgency::Low,
        DiagnosticSeverity::Warning => notify_rust::Urgency::Normal,
        DiagnosticSeverity::Error => notify_rust::Urgency::Critical,
    };

    let _ = notify_rust::Notification::new()
        .summary(&item.title)
        .body(&item.detail)
        .urgency(urgency)
        .icon("network-wireless-offline")
        .show();
}
