// WebSocket Server for Chrome Extension Communication
// Task 6: WebSocket Server Implementation

use anyhow::{anyhow, Result};
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::handshake::server::{ErrorResponse, Request, Response},
};

/// WebSocket message types for Chrome extension communication
/// All messages include: messageId, sessionId, timestamp for traceability
/// JSON fields are serialized in camelCase for Chrome extension compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    /// Connection established
    #[serde(rename = "connected")]
    Connected {
        #[serde(rename = "messageId")]
        message_id: String,
        #[serde(rename = "sessionId")]
        session_id: String,
        timestamp: u64,
    },

    /// Transcription result
    #[serde(rename = "transcription")]
    Transcription {
        #[serde(rename = "messageId")]
        message_id: String,
        #[serde(rename = "sessionId")]
        session_id: String,
        text: String,
        timestamp: u64,
        /// Optional: Is this a partial (interim) result?
        /// STT-REQ-008.1: New field for MVP1
        #[serde(rename = "isPartial", skip_serializing_if = "Option::is_none")]
        is_partial: Option<bool>,
        /// Optional: Confidence score (0.0-1.0)
        /// STT-REQ-008.1: New field for MVP1
        #[serde(skip_serializing_if = "Option::is_none")]
        confidence: Option<f64>,
        /// Optional: Language code (e.g., "ja", "en")
        /// STT-REQ-008.1: New field for MVP1
        #[serde(skip_serializing_if = "Option::is_none")]
        language: Option<String>,
        /// Optional: Processing time in milliseconds
        /// STT-REQ-008.1: New field for MVP1
        #[serde(rename = "processingTimeMs", skip_serializing_if = "Option::is_none")]
        processing_time_ms: Option<u64>,
    },

    /// Error message
    #[serde(rename = "error")]
    Error {
        #[serde(rename = "messageId")]
        message_id: String,
        #[serde(rename = "sessionId")]
        session_id: String,
        message: String,
        timestamp: u64,
    },

    /// Notification message (STT-REQ-006.9: Model change, upgrade proposal, etc.)
    #[serde(rename = "notification")]
    Notification {
        #[serde(rename = "messageId")]
        message_id: String,
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "notificationType")]
        notification_type: String,
        message: String,
        timestamp: u64,
        /// Optional: Additional data (e.g., old_model, new_model, reason)
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
    },

    /// Google Docs sync status (DOCS-REQ-007: Phase 4)
    #[serde(rename = "docsSync")]
    DocsSync {
        event: DocsSyncEventType,
        #[serde(rename = "documentId", skip_serializing_if = "Option::is_none")]
        document_id: Option<String>,
        #[serde(rename = "queueSize", skip_serializing_if = "Option::is_none")]
        queue_size: Option<u32>,
        #[serde(rename = "errorMessage", skip_serializing_if = "Option::is_none")]
        error_message: Option<String>,
        timestamp: u64,
    },
}

/// Google Docs sync event types (DOCS-REQ-007: Phase 4)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DocsSyncEventType {
    DocsSyncStarted,
    DocsSyncSuccess,
    DocsSyncError,
    DocsSyncOffline,
    DocsSyncOnline,
    DocsSyncQueueUpdate,
}

type WsWriter = SplitSink<WebSocketStream<TcpStream>, Message>;

/// WebSocket connection handle
struct WebSocketConnection {
    writer: Arc<Mutex<WsWriter>>,
}

/// WebSocket server for Chrome extension communication
pub struct WebSocketServer {
    port: Option<u16>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    server_handle: Option<JoinHandle<()>>,
    connections: Arc<Mutex<Vec<Arc<WebSocketConnection>>>>,
    session_id: String,
    message_id_counter: Arc<std::sync::atomic::AtomicU64>,
    app_handle: Option<AppHandle>,
}

impl WebSocketServer {
    pub fn new() -> Self {
        Self {
            port: None,
            shutdown_tx: None,
            server_handle: None,
            connections: Arc::new(Mutex::new(Vec::new())),
            session_id: uuid::Uuid::new_v4().to_string(),
            message_id_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            app_handle: None,
        }
    }

    pub fn new_with_app_handle(app_handle: AppHandle) -> Self {
        let mut server = Self::new();
        server.app_handle = Some(app_handle);
        server
    }

    /// Get current timestamp in milliseconds
    fn timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// Start the WebSocket server
    /// Tries ports 9001-9100 until one succeeds
    pub async fn start(&mut self) -> Result<u16> {
        // Try ports 9001-9100
        for port in 9001..=9100 {
            match self.try_start_on_port(port).await {
                Ok(()) => {
                    self.port = Some(port);
                    return Ok(port);
                }
                Err(_) => continue,
            }
        }

        Err(anyhow!("No available ports in range 9001-9100"))
    }

    /// Try to start server on a specific port
    async fn try_start_on_port(&mut self, port: u16) -> Result<()> {
        let addr = format!("127.0.0.1:{}", port);
        let listener = TcpListener::bind(&addr).await?;

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        let connections = Arc::clone(&self.connections);
        let session_id = self.session_id.clone();
        let message_id_counter = Arc::clone(&self.message_id_counter);
        let app_handle = self.app_handle.clone();

        // Spawn server task
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    accept_result = listener.accept() => {
                        if let Ok((stream, _)) = accept_result {
                            let conn_list = Arc::clone(&connections);
                            let sess_id = session_id.clone();
                            let msg_counter = Arc::clone(&message_id_counter);
                            let app_clone = app_handle.clone();
                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(stream, conn_list, sess_id, msg_counter, app_clone).await {
                                    eprintln!("WebSocket connection error: {:?}", e);
                                }
                            });
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });

        self.shutdown_tx = Some(shutdown_tx);
        self.server_handle = Some(handle);

        Ok(())
    }

    /// Verify Origin header (AC-NFR-SEC.2)
    fn verify_origin(origin: &str) -> bool {
        // Allow empty origin (for local testing clients that don't send Origin)
        if origin.is_empty() {
            return true;
        }

        // Allow localhost
        if origin.starts_with("http://127.0.0.1")
            || origin.starts_with("http://localhost")
            || origin.starts_with("https://127.0.0.1")
            || origin.starts_with("https://localhost")
        {
            return true;
        }

        // Allow Google Meet (for Content Script execution context)
        if origin.starts_with("https://meet.google.com") {
            return true;
        }

        // Allow Chrome extensions (development: all, production: configured list)
        if origin.starts_with("chrome-extension://") {
            #[cfg(debug_assertions)]
            return true; // Development: allow all extension IDs

            #[cfg(not(debug_assertions))]
            {
                // Production: check against configured allowed IDs
                // Configure via env: ALLOWED_EXTENSION_IDS="id1,id2"
                let allowed_ids = std::env::var("ALLOWED_EXTENSION_IDS").unwrap_or_default();
                let allowed_ids: Vec<String> = allowed_ids
                    .split(',')
                    .map(|id| id.trim().to_string())
                    .filter(|id| !id.is_empty())
                    .collect();

                if allowed_ids.is_empty() {
                    eprintln!(
                        "[WebSocket] WARNING: ALLOWED_EXTENSION_IDS not set; allowing all chrome-extension origins in release"
                    );
                    return true;
                }

                return allowed_ids
                    .iter()
                    .any(|id| origin.starts_with(&format!("chrome-extension://{}", id)));
            }
        }

        false
    }

    /// Handle a WebSocket connection
    async fn handle_connection(
        stream: TcpStream,
        connections: Arc<Mutex<Vec<Arc<WebSocketConnection>>>>,
        session_id: String,
        message_id_counter: Arc<std::sync::atomic::AtomicU64>,
        app_handle: Option<AppHandle>,
    ) -> Result<()> {
        // Accept with Origin header validation
        let ws_stream = accept_hdr_async(stream, |req: &Request, response: Response| {
            // Get Origin header
            let origin = req
                .headers()
                .get("Origin")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            // Verify origin
            if !Self::verify_origin(origin) {
                eprintln!("Rejected connection from invalid Origin: {}", origin);
                // Return 403 Forbidden using ErrorResponse to properly reject the handshake
                return Err(ErrorResponse::new(Some("Invalid Origin".to_string())));
            }

            Ok(response)
        })
        .await?;
        let (writer, mut reader) = ws_stream.split();

        let conn = Arc::new(WebSocketConnection {
            writer: Arc::new(Mutex::new(writer)),
        });

        // Add to connection list
        {
            let mut conns = connections.lock().await;
            conns.push(Arc::clone(&conn));
        }

        // Send connected message with all required fields
        let message_id = {
            let id = message_id_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            format!("ws-{}", id)
        };

        let connected_msg = WebSocketMessage::Connected {
            message_id,
            session_id,
            timestamp: Self::timestamp(),
        };
        let json = serde_json::to_string(&connected_msg)?;
        let mut writer_guard = conn.writer.lock().await;
        writer_guard.send(Message::Text(json)).await?;
        drop(writer_guard);

        // Read messages (keep-alive + docsSync events from Chrome extension)
        while let Some(msg) = reader.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // Try to parse as WebSocketMessage
                    match serde_json::from_str::<WebSocketMessage>(&text) {
                        Ok(WebSocketMessage::DocsSync { event, document_id, queue_size, error_message, timestamp }) => {
                            // Log docsSync events for monitoring
                            println!(
                                r#"{{"event":"docs_sync","sync_event":"{:?}","document_id":{:?},"queue_size":{:?},"error":{:?},"timestamp":{}}}"#,
                                event,
                                document_id,
                                queue_size,
                                error_message,
                                timestamp
                            );

                            if let Some(app) = app_handle.as_ref() {
                                let payload = serde_json::json!({
                                    "event": event,
                                    "document_id": document_id,
                                    "queue_size": queue_size,
                                    "error_message": error_message,
                                    "timestamp": timestamp,
                                });

                                if let Err(e) = app.emit("docs_sync", payload) {
                                    eprintln!("[WebSocket] Failed to emit docs_sync event: {:?}", e);
                                }
                            }
                        }
                        Ok(_) => {
                            // Other message types - log for debugging
                            println!("[WebSocket] Received message: {}", text);
                        }
                        Err(e) => {
                            eprintln!("[WebSocket] Failed to parse message: {} - {}", e, text);
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break,
                _ => {} // Ignore ping/pong/binary
            }
        }

        // Remove from connection list
        {
            let mut conns = connections.lock().await;
            conns.retain(|c| !Arc::ptr_eq(c, &conn));
        }

        Ok(())
    }

    /// Broadcast a message to all connected clients
    /// Includes performance metrics logging (AC-NFR-PERF.4)
    pub async fn broadcast(&self, message: WebSocketMessage) -> Result<()> {
        let start = std::time::Instant::now();

        let json = serde_json::to_string(&message)?;
        let msg = Message::Text(json);

        let conns = self.connections.lock().await;
        let conn_count = conns.len();

        for conn in conns.iter() {
            let mut writer = conn.writer.lock().await;
            if let Err(e) = writer.send(msg.clone()).await {
                eprintln!("Broadcast send error: {:?}", e);
            }
        }

        // Log performance metrics (AC-NFR-PERF.4)
        let elapsed_ms = start.elapsed().as_millis() as u64;
        println!(
            r#"{{"metric":"websocket_broadcast_ms","value":{},"timestamp":{},"session_id":"{}","connections":{}}}"#,
            elapsed_ms,
            Self::timestamp(),
            self.session_id,
            conn_count
        );

        Ok(())
    }

    /// Stop the WebSocket server
    pub async fn stop(&mut self) -> Result<()> {
        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        // Wait for server task to finish
        if let Some(handle) = self.server_handle.take() {
            let _ = handle.await;
        }

        // Clear connections
        {
            let mut conns = self.connections.lock().await;
            conns.clear();
        }

        self.port = None;

        Ok(())
    }
}
