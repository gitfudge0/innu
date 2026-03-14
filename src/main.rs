use std::sync::Arc;

use anyhow::Context;
use eframe::egui;
use inno::app::WifiApp;
use inno::backend::nm::BackendController;
use inno::platform::tray::TrayBridge;
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("inno=info,ksni=warn")),
        )
        .init();

    let controller = Arc::new(
        BackendController::spawn().context("failed to start NetworkManager backend worker")?,
    );
    let tray_bridge = TrayBridge::spawn(controller.command_sender()).ok();

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([980.0, 700.0])
        .with_min_inner_size([280.0, 420.0])
        .with_title("Inno Wi-Fi");

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Inno Wi-Fi",
        native_options,
        Box::new(move |cc| Ok(Box::new(WifiApp::new(cc, controller, tray_bridge)))),
    )
    .map_err(|error| anyhow::anyhow!("{error}"))
}
