use std::fs;
use std::path::PathBuf;

use directories::BaseDirs;

#[derive(Debug, Clone)]
pub struct AutostartManager {
    desktop_id: &'static str,
    app_name: &'static str,
}

impl AutostartManager {
    pub fn new(desktop_id: &'static str, app_name: &'static str) -> Self {
        Self {
            desktop_id,
            app_name,
        }
    }

    pub fn sync(&self, enabled: bool) -> anyhow::Result<()> {
        let path = self.desktop_entry_path()?;
        if enabled {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let exec = std::env::current_exe()?;
            let entry = format!(
                "[Desktop Entry]\nType=Application\nVersion=1.0\nName={}\nExec={}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
                self.app_name,
                exec.display()
            );
            fs::write(path, entry)?;
        } else if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    fn desktop_entry_path(&self) -> anyhow::Result<PathBuf> {
        let base_dirs =
            BaseDirs::new().ok_or_else(|| anyhow::anyhow!("failed to resolve HOME directory"))?;
        Ok(base_dirs
            .config_dir()
            .join("autostart")
            .join(format!("{}.desktop", self.desktop_id)))
    }
}
