// WebSocket Server for Chrome Extension Communication
// Walking Skeleton (MVP0) - Empty Implementation

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// WebSocket message types for Chrome extension communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    /// Connection established
    #[serde(rename = "connected")]
    Connected { session_id: String },

    /// Transcription result
    #[serde(rename = "transcription")]
    Transcription { text: String, timestamp: u64 },

    /// Error message
    #[serde(rename = "error")]
    Error { message: String },
}

/// WebSocket server for Chrome extension communication
pub struct WebSocketServer;

impl WebSocketServer {
    pub fn new() -> Self {
        Self
    }

    /// Start the WebSocket server
    pub async fn start(&mut self) -> Result<u16> {
        unimplemented!("WebSocketServer::start - to be implemented in Task 6.1")
    }

    /// Broadcast a message to all connected clients
    pub async fn broadcast(&self, _message: WebSocketMessage) -> Result<()> {
        unimplemented!("WebSocketServer::broadcast - to be implemented in Task 6.2")
    }

    /// Stop the WebSocket server
    pub async fn stop(&mut self) -> Result<()> {
        unimplemented!("WebSocketServer::stop - to be implemented in Task 8.3")
    }
}
