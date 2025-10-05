// Tauri Commands
// Walking Skeleton (MVP0) - Empty Implementation

use crate::state::AppState;
use tauri::State;

/// Start recording command
#[tauri::command]
pub async fn start_recording(_state: State<'_, AppState>) -> Result<String, String> {
    unimplemented!("start_recording command - to be implemented in Task 2.2")
}

/// Stop recording command
#[tauri::command]
pub async fn stop_recording(_state: State<'_, AppState>) -> Result<String, String> {
    unimplemented!("stop_recording command - to be implemented in Task 2.2")
}
