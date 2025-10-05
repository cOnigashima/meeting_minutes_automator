// Integration Test: WebSocket Server Connection and Broadcast
// Tests WebSocket server with actual client connections

use meeting_minutes_automator_lib::websocket::{WebSocketServer, WebSocketMessage};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::StreamExt;
use tokio::time::{Duration, timeout};

#[tokio::test]
async fn it_websocket_server_client_connection() {
    // Test: Client can connect and receive connected message
    let mut server = WebSocketServer::new();
    let port = server.start().await.expect("Should start server");

    // Connect client
    let url = format!("ws://127.0.0.1:{}", port);
    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut _write, mut read) = ws_stream.split();

    // Should receive connected message
    let msg = timeout(Duration::from_secs(1), read.next())
        .await
        .expect("Timeout")
        .expect("No message")
        .expect("Error in message");

    if let Message::Text(text) = msg {
        let json: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
        assert_eq!(json.get("type").and_then(|v| v.as_str()), Some("connected"));
        assert!(json.get("session_id").is_some(), "session_id missing");
        assert!(json.get("message_id").is_some(), "message_id missing");
        assert!(json.get("timestamp").is_some(), "timestamp missing");
    } else {
        panic!("Expected text message");
    }

    server.stop().await.expect("Should stop server");
}

#[tokio::test]
async fn it_websocket_server_broadcast() {
    // Test: Broadcast message reaches all connected clients
    let mut server = WebSocketServer::new();
    let port = server.start().await.expect("Should start server");

    // Connect two clients
    let url = format!("ws://127.0.0.1:{}", port);

    let (ws_stream1, _) = connect_async(&url).await.expect("Failed to connect client 1");
    let (mut _write1, mut read1) = ws_stream1.split();

    let (ws_stream2, _) = connect_async(&url).await.expect("Failed to connect client 2");
    let (mut _write2, mut read2) = ws_stream2.split();

    // Skip connected messages
    let _ = read1.next().await;
    let _ = read2.next().await;

    // Broadcast transcription message
    let broadcast_msg = WebSocketMessage::Transcription {
        message_id: "test-msg-1".to_string(),
        session_id: "test-session".to_string(),
        text: "Test transcription".to_string(),
        timestamp: 12345,
    };

    server.broadcast(broadcast_msg).await.expect("Should broadcast");

    // Both clients should receive the message
    for (i, read) in [&mut read1, &mut read2].iter_mut().enumerate() {
        let msg = timeout(Duration::from_secs(1), read.next())
            .await
            .expect("Timeout")
            .expect("No message")
            .expect("Error in message");

        if let Message::Text(text) = msg {
            let json: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
            assert_eq!(json.get("type").and_then(|v| v.as_str()), Some("transcription"));
            assert_eq!(json.get("text").and_then(|v| v.as_str()), Some("Test transcription"));
            println!("✅ Client {} received broadcast message", i + 1);
        } else {
            panic!("Expected text message");
        }
    }

    server.stop().await.expect("Should stop server");
}

#[tokio::test]
async fn it_websocket_server_multiple_broadcasts() {
    // Test: Multiple broadcasts work correctly
    let mut server = WebSocketServer::new();
    let port = server.start().await.expect("Should start server");

    let url = format!("ws://127.0.0.1:{}", port);
    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    let (mut _write, mut read) = ws_stream.split();

    // Skip connected message
    let _ = read.next().await;

    // Send 3 broadcasts
    for i in 0..3 {
        let msg = WebSocketMessage::Transcription {
            message_id: format!("msg-{}", i),
            session_id: "test-session".to_string(),
            text: format!("Message {}", i),
            timestamp: i as u64,
        };
        server.broadcast(msg).await.expect("Should broadcast");
    }

    // Receive all 3 messages
    for i in 0..3 {
        let msg = timeout(Duration::from_secs(1), read.next())
            .await
            .expect("Timeout")
            .expect("No message")
            .expect("Error in message");

        if let Message::Text(text) = msg {
            let json: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
            let expected = format!("Message {}", i);
            assert_eq!(json.get("text").and_then(|v| v.as_str()), Some(expected.as_str()));
            println!("✅ Received message {}", i);
        }
    }

    server.stop().await.expect("Should stop server");
}
