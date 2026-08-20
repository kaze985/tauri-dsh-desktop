use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

/// What happens when the user closes the main window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CloseAction {
    /// Ask every time (default).
    #[default]
    Ask,
    /// Minimize to tray.
    Tray,
    /// Exit the application.
    Exit,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Prefs {
    #[serde(default)]
    pub close_action: CloseAction,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            close_action: CloseAction::Ask,
        }
    }
}

fn prefs_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join("prefs.json"))
}

/// Load preferences from disk; falls back to defaults on any error.
pub fn load(app: &AppHandle) -> Prefs {
    let Some(path) = prefs_path(app) else {
        return Prefs::default();
    };
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist preferences to disk (best effort).
pub fn save(app: &AppHandle, prefs: &Prefs) {
    if let Some(path) = prefs_path(app) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(
            path,
            serde_json::to_string_pretty(prefs).unwrap_or_default(),
        );
    }
}
