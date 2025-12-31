// Application State Management
// Walking Skeleton (MVP0) - WebSocket Server Integration
// MVP1 - Audio Device Event Management
// Task 10.4 Phase 2 - Device Reconnection Management

use crate::audio_device_adapter::{AudioDeviceAdapter, AudioEventReceiver, AudioEventSender};
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

    /// Selected audio device ID
    /// Task 9.1 - STT-REQ-001.2 (user device selection)
    pub selected_device_id: Mutex<Option<String>>,

    /// WebSocket server for Chrome extension communication
    /// Initialized during Tauri setup, None before initialization
    pub websocket_server: Mutex<Option<Arc<tokio::sync::Mutex<WebSocketServer>>>>,

    /// Python sidecar process manager
    /// Initialized during Tauri setup, None before initialization
    pub python_sidecar: Mutex<Option<Arc<tokio::sync::Mutex<PythonSidecarManager>>>>,

    /// Audio device adapter for real/fake audio capture
    /// Initialized during Tauri setup, None before initialization
    pub audio_device: Mutex<Option<Arc<tokio::sync::Mutex<Box<dyn AudioDeviceAdapter>>>>>,

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
            websocket_server: Mutex::new(None),
            python_sidecar: Mutex::new(None),
            audio_device: Mutex::new(None),
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
