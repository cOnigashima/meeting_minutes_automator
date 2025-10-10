// Application State Management
// Walking Skeleton (MVP0) - WebSocket Server Integration
// MVP1 - Audio Device Event Management

use std::sync::{Arc, Mutex};
use crate::websocket::WebSocketServer;
use crate::python_sidecar::PythonSidecarManager;
use crate::audio::FakeAudioDevice;
use crate::audio_device_adapter::{AudioEventReceiver, AudioEventSender};

/// Application state shared across Tauri commands
pub struct AppState {
    /// Recording state
    pub is_recording: Mutex<bool>,

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
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_recording: Mutex::new(false),
            websocket_server: Mutex::new(None),
            python_sidecar: Mutex::new(None),
            audio_device: Mutex::new(None),
            audio_event_tx: Mutex::new(None),
            audio_event_rx: Mutex::new(None),
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
}
