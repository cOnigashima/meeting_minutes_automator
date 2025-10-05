// Application State Management
// Walking Skeleton (MVP0) - WebSocket Server Integration

use std::sync::{Arc, Mutex};
use crate::websocket::WebSocketServer;

/// Application state shared across Tauri commands
pub struct AppState {
    /// Recording state
    pub is_recording: Mutex<bool>,

    /// WebSocket server for Chrome extension communication
    /// Initialized during Tauri setup, None before initialization
    pub websocket_server: Mutex<Option<Arc<tokio::sync::Mutex<WebSocketServer>>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_recording: Mutex::new(false),
            websocket_server: Mutex::new(None),
        }
    }

    /// Set WebSocket server after initialization
    pub fn set_websocket_server(&self, server: Arc<tokio::sync::Mutex<WebSocketServer>>) {
        let mut ws = self.websocket_server.lock().unwrap();
        *ws = Some(server);
    }
}
