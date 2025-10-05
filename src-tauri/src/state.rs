// Application State Management
// Walking Skeleton (MVP0) - Empty Implementation

use std::sync::Mutex;

/// Application state shared across Tauri commands
pub struct AppState {
    /// Recording state
    pub is_recording: Mutex<bool>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_recording: Mutex::new(false),
        }
    }
}
