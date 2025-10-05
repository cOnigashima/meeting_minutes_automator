// WebSocket Server for Chrome Extension Communication
// Task 6: WebSocket Server Implementation

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};
use futures_util::stream::SplitSink;
use tokio_tungstenite::WebSocketStream;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;

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
}

impl WebSocketServer {
    pub fn new() -> Self {
        Self {
            port: None,
            shutdown_tx: None,
            server_handle: None,
            connections: Arc::new(Mutex::new(Vec::new())),
        }
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

        // Spawn server task
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    accept_result = listener.accept() => {
                        if let Ok((stream, _)) = accept_result {
                            let conn_list = Arc::clone(&connections);
                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(stream, conn_list).await {
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

    /// Handle a WebSocket connection
    async fn handle_connection(
        stream: TcpStream,
        connections: Arc<Mutex<Vec<Arc<WebSocketConnection>>>>,
    ) -> Result<()> {
        let ws_stream = accept_async(stream).await?;
        let (writer, mut reader) = ws_stream.split();

        let conn = Arc::new(WebSocketConnection {
            writer: Arc::new(Mutex::new(writer)),
        });

        // Add to connection list
        {
            let mut conns = connections.lock().await;
            conns.push(Arc::clone(&conn));
        }

        // Send connected message
        let connected_msg = WebSocketMessage::Connected {
            session_id: uuid::Uuid::new_v4().to_string(),
        };
        let json = serde_json::to_string(&connected_msg)?;
        let mut writer_guard = conn.writer.lock().await;
        writer_guard.send(Message::Text(json)).await?;
        drop(writer_guard);

        // Read messages (for keep-alive)
        while let Some(msg) = reader.next().await {
            match msg {
                Ok(Message::Close(_)) => break,
                Err(_) => break,
                _ => {} // Ignore other messages for now
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
    pub async fn broadcast(&self, message: WebSocketMessage) -> Result<()> {
        let json = serde_json::to_string(&message)?;
        let msg = Message::Text(json);

        let conns = self.connections.lock().await;
        for conn in conns.iter() {
            let mut writer = conn.writer.lock().await;
            let _ = writer.send(msg.clone()).await;
        }

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
