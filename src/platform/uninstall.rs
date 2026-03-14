use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use directories::{BaseDirs, ProjectDirs};

const BIN_NAME: &str = env!("CARGO_PKG_NAME");
const APP_NAME: &str = "Innu";

#[derive(Debug, Clone)]
struct UninstallTargets {
    binary_path: PathBuf,
    desktop_entry_path: PathBuf,
    autostart_entry_path: PathBuf,
    config_dir: PathBuf,
}

pub fn run_cli() -> anyhow::Result<()> {
    let targets = uninstall_targets()?;

    println!("This will remove the following {APP_NAME} files if present:\n");
    println!("- {}", targets.binary_path.display());
    println!("- {}", targets.desktop_entry_path.display());
    println!("- {}", targets.autostart_entry_path.display());
    println!("- {}", targets.config_dir.display());
    println!();
    print!("Type 'uninstall' to continue: ");
    io::stdout().flush()?;

    let mut confirmation = String::new();
    io::stdin().read_line(&mut confirmation)?;

    if confirmation.trim() != "uninstall" {
        println!("Cancelled.");
        return Ok(());
    }

    let mut removed = Vec::new();
    let mut missing = Vec::new();

    remove_file_if_present(&targets.binary_path, &mut removed, &mut missing)?;
    remove_file_if_present(&targets.desktop_entry_path, &mut removed, &mut missing)?;
    remove_file_if_present(&targets.autostart_entry_path, &mut removed, &mut missing)?;
    remove_dir_if_present(&targets.config_dir, &mut removed, &mut missing)?;

    if removed.is_empty() {
        println!("No installed {APP_NAME} files were found.");
    } else {
        println!("\nRemoved:");
        for path in &removed {
            println!("- {path}");
        }
    }

    if !missing.is_empty() {
        println!("\nAlready absent:");
        for path in &missing {
            println!("- {path}");
        }
    }

    Ok(())
}

fn uninstall_targets() -> anyhow::Result<UninstallTargets> {
    let base_dirs =
        BaseDirs::new().ok_or_else(|| anyhow::anyhow!("failed to resolve HOME directory"))?;
    let project_dirs = ProjectDirs::from("dev", "gitfudge", BIN_NAME)
        .ok_or_else(|| anyhow::anyhow!("failed to resolve XDG configuration directory"))?;

    Ok(UninstallTargets {
        binary_path: base_dirs.home_dir().join(".local/bin").join(BIN_NAME),
        desktop_entry_path: base_dirs
            .home_dir()
            .join(".local/share/applications")
            .join(format!("{BIN_NAME}.desktop")),
        autostart_entry_path: base_dirs
            .config_dir()
            .join("autostart")
            .join(format!("{BIN_NAME}.desktop")),
        config_dir: project_dirs.config_dir().to_path_buf(),
    })
}

fn remove_file_if_present(
    path: &PathBuf,
    removed: &mut Vec<String>,
    missing: &mut Vec<String>,
) -> anyhow::Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
        removed.push(path.display().to_string());
    } else {
        missing.push(path.display().to_string());
    }

    Ok(())
}

fn remove_dir_if_present(
    path: &PathBuf,
    removed: &mut Vec<String>,
    missing: &mut Vec<String>,
) -> anyhow::Result<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
        removed.push(path.display().to_string());
    } else {
        missing.push(path.display().to_string());
    }

    Ok(())
}
