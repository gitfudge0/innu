use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub tray_enabled: bool,
    pub notifications_enabled: bool,
    pub autostart_enabled: bool,
    pub rescan_while_open: bool,
    pub compact_sidebar: bool,
    pub follow_system_theme: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            tray_enabled: true,
            notifications_enabled: true,
            autostart_enabled: false,
            rescan_while_open: true,
            compact_sidebar: false,
            follow_system_theme: true,
        }
    }
}

impl AppSettings {
    pub fn load() -> anyhow::Result<Self> {
        let path = settings_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(path)?;
        let settings = toml::from_str::<Self>(&raw)?;
        Ok(settings)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = settings_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

pub fn settings_path() -> anyhow::Result<PathBuf> {
    let project_dirs = ProjectDirs::from("dev", "gitfudge", "inno")
        .ok_or_else(|| anyhow::anyhow!("failed to resolve XDG configuration directory"))?;
    Ok(project_dirs.config_dir().join("settings.toml"))
}
