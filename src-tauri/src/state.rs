// Application State Management
// Walking Skeleton (MVP0) - WebSocket Server Integration
// MVP1 - Audio Device Event Management

use crate::audio::FakeAudioDevice;
use crate::audio_device_adapter::{AudioEventReceiver, AudioEventSender};
use crate::python_sidecar::PythonSidecarManager;
use crate::websocket::WebSocketServer;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

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

    /// Fake audio device for testing
    /// Initialized during Tauri setup, None before initialization
    pub audio_device: Mutex<Option<Arc<tokio::sync::Mutex<FakeAudioDevice>>>>,

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
        }
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
    pub fn set_audio_device(&self, device: Arc<tokio::sync::Mutex<FakeAudioDevice>>) {
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
