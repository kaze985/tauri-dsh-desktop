use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crate::prefs::Prefs;

/// Shared application state managed by Tauri.
pub struct AppState {
    /// PID of the spawned dsh service process tree (the cmd.exe wrapper).
    pub pid: Mutex<Option<u32>>,
    /// The authenticated dsh web URL (carries ?token=) parsed from stdout.
    pub web_url: Arc<Mutex<Option<String>>>,
    /// True once the user chose to exit; suppresses crash UI during shutdown.
    pub shutting_down: AtomicBool,
    /// True while an upgrade is in flight; suppresses crash UI during restarts.
    pub upgrading: AtomicBool,
    /// Persisted user preferences (close behavior).
    pub prefs: Mutex<Prefs>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            pid: Mutex::new(None),
            web_url: Arc::new(Mutex::new(None)),
            shutting_down: AtomicBool::new(false),
            upgrading: AtomicBool::new(false),
            prefs: Mutex::new(Prefs::default()),
        }
    }
}
