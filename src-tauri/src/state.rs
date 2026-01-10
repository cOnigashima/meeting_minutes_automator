// Application State Management
// Walking Skeleton (MVP0) - WebSocket Server Integration
// MVP1 - Audio Device Event Management
// Task 10.4 Phase 2 - Device Reconnection Management

use crate::audio_device_adapter::{AudioDeviceAdapter, AudioEventReceiver, AudioEventSender};
use crate::audio_device_recorder::AudioDeviceRecorder;
use crate::python_sidecar::PythonSidecarManager;
use crate::reconnection_manager::ReconnectionManager;
use crate::websocket::WebSocketServer;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

/// Type alias for Python sidecar stdin handle
pub type SidecarStdin = Arc<tokio::sync::Mutex<tokio::process::ChildStdin>>;
/// Type alias for Python sidecar stdout handle
pub type SidecarStdout =
    Arc<tokio::sync::Mutex<tokio::io::BufReader<tokio::process::ChildStdout>>>;

/// Application state shared across Tauri commands
pub struct AppState {
    /// Recording state
    pub is_recording: Mutex<bool>,

    /// Selected audio device ID (single device mode - backward compatible)
    /// Task 9.1 - STT-REQ-001.2 (user device selection)
    pub selected_device_id: Mutex<Option<String>>,

    /// Selected audio device IDs (multi-input mode)
    /// STTMIX Task 1.2 - STTMIX-REQ-001.2 (multi-device selection)
    pub selected_device_ids: Mutex<Vec<String>>,

    /// Multi-input mode enabled flag
    /// STTMIX Task 1.2 - STTMIX-REQ-001
    pub multi_input_enabled: Mutex<bool>,

    /// WebSocket server for Chrome extension communication
    /// Initialized during Tauri setup, None before initialization
    pub websocket_server: Mutex<Option<Arc<tokio::sync::Mutex<WebSocketServer>>>>,

    /// Python sidecar process manager
    /// Initialized during Tauri setup, None before initialization
    pub python_sidecar: Mutex<Option<Arc<tokio::sync::Mutex<PythonSidecarManager>>>>,

    /// Audio device adapter for real/fake audio capture
    /// Initialized during Tauri setup, None before initialization
    pub audio_device: Mutex<Option<Arc<tokio::sync::Mutex<Box<dyn AudioDeviceAdapter>>>>>,

    /// Audio device recorder facade (single/multi-input)
    /// Initialized during Tauri setup, None before initialization
    pub audio_recorder: Mutex<Option<Arc<tokio::sync::Mutex<AudioDeviceRecorder>>>>,

    /// Audio device event sender for monitoring device health
    /// MVP1 - STT-REQ-004.9/10/11
    pub audio_event_tx: Mutex<Option<AudioEventSender>>,

    /// Audio device event receiver for monitoring device health
    /// MVP1 - STT-REQ-004.9/10/11
    pub audio_event_rx: Mutex<Option<AudioEventReceiver>>,

    /// Broadcast channel for IPC events from Python sidecar
    /// Solves deadlock: single global reader task distributes events to all listeners
    /// Related: STT-REQ-007 (Event Stream Protocol)
    pub ipc_event_tx: Mutex<Option<broadcast::Sender<serde_json::Value>>>,

    /// Current recording session identifier (UUID v4)
    pub session_id: Mutex<Option<String>>,

    /// Reconnection manager for audio device recovery
    /// Task 10.4 Phase 2 - STT-REQ-004.11
    /// Using tokio::sync::Mutex to allow .await across lock (Send requirement)
    pub reconnection_manager: tokio::sync::Mutex<ReconnectionManager>,

    /// Python sidecar stdin handle (extracted for concurrent access)
    /// Allows audio sender task to write without blocking IPC reader
    pub sidecar_stdin: Mutex<Option<SidecarStdin>>,

    /// Python sidecar stdout handle (extracted for concurrent access)
    /// Allows IPC reader task to read without blocking audio sender
    pub sidecar_stdout: Mutex<Option<SidecarStdout>>,

    /// Cancellation token for recording tasks (IPC reader, audio sender)
    /// Used to gracefully stop tasks when recording ends
    pub recording_cancel_token: Mutex<Option<CancellationToken>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_recording: Mutex::new(false),
            selected_device_id: Mutex::new(None),
            selected_device_ids: Mutex::new(Vec::new()),
            multi_input_enabled: Mutex::new(false),
            websocket_server: Mutex::new(None),
            python_sidecar: Mutex::new(None),
            audio_device: Mutex::new(None),
            audio_recorder: Mutex::new(None),
            audio_event_tx: Mutex::new(None),
            audio_event_rx: Mutex::new(None),
            ipc_event_tx: Mutex::new(None),
            session_id: Mutex::new(None),
            reconnection_manager: tokio::sync::Mutex::new(ReconnectionManager::new()),
            sidecar_stdin: Mutex::new(None),
            sidecar_stdout: Mutex::new(None),
            recording_cancel_token: Mutex::new(None),
        }
    }

    /// Create and store a new cancellation token for recording tasks
    /// Returns a clone of the token for use by tasks
    pub fn create_recording_cancel_token(&self) -> CancellationToken {
        let token = CancellationToken::new();
        *self.recording_cancel_token.lock().unwrap() = Some(token.clone());
        token
    }

    /// Cancel the current recording tasks
    pub fn cancel_recording_tasks(&self) {
        if let Some(token) = self.recording_cancel_token.lock().unwrap().take() {
            token.cancel();
        }
    }

    /// Set sidecar stdin/stdout handles after extraction
    pub fn set_sidecar_handles(&self, stdin: SidecarStdin, stdout: SidecarStdout) {
        *self.sidecar_stdin.lock().unwrap() = Some(stdin);
        *self.sidecar_stdout.lock().unwrap() = Some(stdout);
    }

    /// Get sidecar stdin handle
    pub fn get_sidecar_stdin(&self) -> Option<SidecarStdin> {
        self.sidecar_stdin.lock().unwrap().clone()
    }

    /// Get sidecar stdout handle
    pub fn get_sidecar_stdout(&self) -> Option<SidecarStdout> {
        self.sidecar_stdout.lock().unwrap().clone()
    }

    /// Set WebSocket server after initialization
    pub fn set_websocket_server(&self, server: Arc<tokio::sync::Mutex<WebSocketServer>>) {
        let mut ws = self.websocket_server.lock().unwrap();
        *ws = Some(server);
    }

    /// Set Python sidecar manager after initialization
    pub fn set_python_sidecar(&self, sidecar: Arc<tokio::sync::Mutex<PythonSidecarManager>>) {
        let mut py = self.python_sidecar.lock().unwrap();
        *py = Some(sidecar);
    }

    /// Set audio device after initialization
    pub fn set_audio_device(&self, device: Arc<tokio::sync::Mutex<Box<dyn AudioDeviceAdapter>>>) {
        let mut audio = self.audio_device.lock().unwrap();
        *audio = Some(device);
    }

    /// Set audio recorder after initialization
    pub fn set_audio_recorder(&self, recorder: Arc<tokio::sync::Mutex<AudioDeviceRecorder>>) {
        let mut audio = self.audio_recorder.lock().unwrap();
        *audio = Some(recorder);
    }

    /// Set audio event channel after initialization
    /// MVP1 - STT-REQ-004.9/10/11
    pub fn set_audio_event_channel(&self, tx: AudioEventSender, rx: AudioEventReceiver) {
        let mut tx_lock = self.audio_event_tx.lock().unwrap();
        *tx_lock = Some(tx);
        let mut rx_lock = self.audio_event_rx.lock().unwrap();
        *rx_lock = Some(rx);
    }

    /// Take audio event receiver (can only be called once)
    /// MVP1 - STT-REQ-004.9/10/11
    pub fn take_audio_event_rx(&self) -> Option<AudioEventReceiver> {
        let mut rx_lock = self.audio_event_rx.lock().unwrap();
        rx_lock.take()
    }

    /// Set IPC event broadcast channel
    /// Related: STT-REQ-007 (Event Stream Protocol deadlock fix)
    pub fn set_ipc_event_channel(&self, tx: broadcast::Sender<serde_json::Value>) {
        let mut tx_lock = self.ipc_event_tx.lock().unwrap();
        *tx_lock = Some(tx);
    }

    /// Subscribe to IPC events
    /// Returns a receiver for listening to all Python sidecar events
    pub fn subscribe_ipc_events(&self) -> Option<broadcast::Receiver<serde_json::Value>> {
        let tx_lock = self.ipc_event_tx.lock().unwrap();
        tx_lock.as_ref().map(|tx| tx.subscribe())
    }

    /// Set selected audio device ID
    /// Task 9.1 - STT-REQ-001.2
    pub fn set_selected_device_id(&self, device_id: String) {
        let mut selected = self.selected_device_id.lock().unwrap();
        *selected = Some(device_id);
    }

    /// Get selected audio device ID
    /// Task 9.1 - STT-REQ-001.2
    pub fn get_selected_device_id(&self) -> Option<String> {
        let selected = self.selected_device_id.lock().unwrap();
        selected.clone()
    }

    // ========================================================================
    // Multi-Input Support (STTMIX Task 1.2)
    // ========================================================================

    /// Set multi-input mode enabled/disabled
    /// STTMIX Task 1.2 - STTMIX-REQ-001
    pub fn set_multi_input_enabled(&self, enabled: bool) {
        let mut flag = self.multi_input_enabled.lock().unwrap();
        *flag = enabled;
    }

    /// Check if multi-input mode is enabled
    /// STTMIX Task 1.2 - STTMIX-REQ-001
    pub fn is_multi_input_enabled(&self) -> bool {
        let flag = self.multi_input_enabled.lock().unwrap();
        *flag
    }

    /// Set selected audio device IDs for multi-input mode
    /// STTMIX Task 1.2 - STTMIX-REQ-001.2
    ///
    /// # Arguments
    /// * `device_ids` - Vector of device IDs (max 2 per STTMIX-CON-005)
    pub fn set_selected_device_ids(&self, device_ids: Vec<String>) {
        let mut selected = self.selected_device_ids.lock().unwrap();
        *selected = device_ids;
    }

    /// Get selected audio device IDs for multi-input mode
    /// STTMIX Task 1.2 - STTMIX-REQ-001.2
    pub fn get_selected_device_ids(&self) -> Vec<String> {
        let selected = self.selected_device_ids.lock().unwrap();
        selected.clone()
    }

    /// Clear selected audio device IDs
    /// STTMIX Task 1.2
    pub fn clear_selected_device_ids(&self) {
        let mut selected = self.selected_device_ids.lock().unwrap();
        selected.clear();
    }

    /// Get effective device IDs based on current mode
    /// Returns single device ID wrapped in Vec for single mode,
    /// or multiple device IDs for multi-input mode
    /// STTMIX Task 1.2 - Backward compatible API
    pub fn get_effective_device_ids(&self) -> Vec<String> {
        if self.is_multi_input_enabled() {
            self.get_selected_device_ids()
        } else {
            // Single mode: wrap single device ID in Vec for uniform handling
            self.get_selected_device_id()
                .map(|id| vec![id])
                .unwrap_or_default()
        }
    }

    /// Set active recording session identifier
    pub fn set_session_id(&self, session: String) {
        let mut guard = self.session_id.lock().unwrap();
        *guard = Some(session);
    }

    /// Get current session identifier
    pub fn get_session_id(&self) -> Option<String> {
        let guard = self.session_id.lock().unwrap();
        guard.clone()
    }

    /// Clear current session identifier
    pub fn clear_session_id(&self) {
        let mut guard = self.session_id.lock().unwrap();
        *guard = None;
    }
}

// ============================================================================
// Tests - Multi-Input Support (STTMIX Task 1.2)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_new_defaults() {
        let state = AppState::new();

        // Verify default values for multi-input fields
        assert!(!state.is_multi_input_enabled());
        assert!(state.get_selected_device_ids().is_empty());
    }

    #[test]
    fn test_multi_input_enabled_toggle() {
        let state = AppState::new();

        // Initially disabled
        assert!(!state.is_multi_input_enabled());

        // Enable
        state.set_multi_input_enabled(true);
        assert!(state.is_multi_input_enabled());

        // Disable
        state.set_multi_input_enabled(false);
        assert!(!state.is_multi_input_enabled());
    }

    #[test]
    fn test_selected_device_ids() {
        let state = AppState::new();

        // Initially empty
        assert!(state.get_selected_device_ids().is_empty());

        // Set device IDs
        state.set_selected_device_ids(vec!["mic-1".to_string(), "loopback-1".to_string()]);
        let ids = state.get_selected_device_ids();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], "mic-1");
        assert_eq!(ids[1], "loopback-1");

        // Clear device IDs
        state.clear_selected_device_ids();
        assert!(state.get_selected_device_ids().is_empty());
    }

    #[test]
    fn test_effective_device_ids_single_mode() {
        let state = AppState::new();

        // Single mode (default)
        assert!(!state.is_multi_input_enabled());

        // No device selected - empty
        assert!(state.get_effective_device_ids().is_empty());

        // Single device selected
        state.set_selected_device_id("mic-1".to_string());
        let ids = state.get_effective_device_ids();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], "mic-1");
    }

    #[test]
    fn test_effective_device_ids_multi_mode() {
        let state = AppState::new();

        // Enable multi-input mode
        state.set_multi_input_enabled(true);

        // Set multiple device IDs
        state.set_selected_device_ids(vec!["mic-1".to_string(), "loopback-1".to_string()]);

        // Also set single device ID (should be ignored in multi mode)
        state.set_selected_device_id("ignored-device".to_string());

        // Should return multi-input device IDs
        let ids = state.get_effective_device_ids();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], "mic-1");
        assert_eq!(ids[1], "loopback-1");
    }

    #[test]
    fn test_backward_compatibility_single_device() {
        let state = AppState::new();

        // Existing API should still work
        state.set_selected_device_id("mic-1".to_string());
        assert_eq!(state.get_selected_device_id(), Some("mic-1".to_string()));

        // Multi-input fields should be independent
        assert!(!state.is_multi_input_enabled());
        assert!(state.get_selected_device_ids().is_empty());
    }
}
