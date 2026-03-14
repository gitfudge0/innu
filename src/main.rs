use std::sync::Arc;

use anyhow::Context;
use eframe::egui;
use innu::app::WifiApp;
use innu::backend::nm::BackendController;
use innu::platform::tray::TrayBridge;
use innu::platform::uninstall;
use tracing_subscriber::EnvFilter;

const APP_NAME: &str = "Innu";
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> anyhow::Result<()> {
    match parse_cli(std::env::args().skip(1))? {
        Command::Help => {
            print_help();
            return Ok(());
        }
        Command::Version => {
            println!("{APP_NAME} {VERSION}");
            return Ok(());
        }
        Command::Uninstall => {
            return uninstall::run_cli();
        }
        Command::Run => {}
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("innu=info,ksni=warn")),
        )
        .init();

    let controller = Arc::new(
        BackendController::spawn().context("failed to start NetworkManager backend worker")?,
    );
    let tray_bridge = TrayBridge::spawn(controller.command_sender()).ok();

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([980.0, 700.0])
        .with_min_inner_size([280.0, 420.0])
        .with_title("Innu - Network Management");

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Innu - Network Management",
        native_options,
        Box::new(move |cc| Ok(Box::new(WifiApp::new(cc, controller, tray_bridge)))),
    )
    .map_err(|error| anyhow::anyhow!("{error}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Run,
    Help,
    Version,
    Uninstall,
}

fn parse_cli(args: impl Iterator<Item = String>) -> anyhow::Result<Command> {
    let mut args = args;

    match args.next().as_deref() {
        None => Ok(Command::Run),
        Some("--help") | Some("-h") => ensure_no_extra_args(args, Command::Help),
        Some("--version") | Some("-v") => ensure_no_extra_args(args, Command::Version),
        Some("uninstall") => ensure_no_extra_args(args, Command::Uninstall),
        Some(other) => Err(anyhow::anyhow!(
            "unrecognized command or option: {other}\n\n{}",
            help_text()
        )),
    }
}

fn ensure_no_extra_args(
    mut args: impl Iterator<Item = String>,
    command: Command,
) -> anyhow::Result<Command> {
    if let Some(extra) = args.next() {
        Err(anyhow::anyhow!(
            "unexpected argument: {extra}\n\n{}",
            help_text()
        ))
    } else {
        Ok(command)
    }
}

fn print_help() {
    println!("{}", help_text());
}

fn help_text() -> String {
    format!(
        "{APP_NAME} {VERSION}\n\nUsage:\n  innu\n  innu uninstall\n  innu --help\n  innu --version\n\nOptions:\n  -h, --help       Show this help message\n  -v, --version    Show the installed version\n\nCommands:\n  uninstall        Remove the installed binary, desktop entry, autostart entry, and app config"
    )
}
