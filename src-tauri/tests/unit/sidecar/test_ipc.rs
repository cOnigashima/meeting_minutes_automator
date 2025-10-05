// Unit Tests for IPC Communication
// Tests for JSON message send/receive

use meeting_minutes_automator_lib::python_sidecar::PythonSidecarManager;

#[tokio::test]
async fn ut_4_1_1_ping_pong_echo() {
    // Test: Should send ping and receive pong

    let mut manager = PythonSidecarManager::new();
    manager.start().await.expect("Process should start");
    manager.wait_for_ready().await.expect("Should receive ready");

    // Send ping
    let ping_msg = serde_json::json!({
        "type": "ping",
        "id": "test-ping-1"
    });

    manager.send_message(ping_msg).await.expect("Should send ping");

    // Receive pong
    let response = manager.receive_message().await.expect("Should receive pong");

    println!("✅ Received response: {:?}", response);

    assert_eq!(response.get("type").and_then(|v| v.as_str()), Some("pong"), "Should receive pong type");
    assert_eq!(response.get("id").and_then(|v| v.as_str()), Some("test-ping-1"), "Should match request ID");

    // Cleanup
    let _ = manager.shutdown().await;
}

#[tokio::test]
async fn ut_4_1_2_process_audio_fake_response() {
    // Test: Should send process_audio and receive fake transcription

    let mut manager = PythonSidecarManager::new();
    manager.start().await.expect("Process should start");
    manager.wait_for_ready().await.expect("Should receive ready");

    // Send process_audio request
    let audio_msg = serde_json::json!({
        "type": "process_audio",
        "id": "test-audio-1",
        "audio_data": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
    });

    manager.send_message(audio_msg).await.expect("Should send audio message");

    // Receive transcription result
    let response = manager.receive_message().await.expect("Should receive transcription");

    println!("✅ Received transcription: {:?}", response);

    assert_eq!(
        response.get("type").and_then(|v| v.as_str()),
        Some("transcription_result"),
        "Should receive transcription_result type"
    );
    assert_eq!(
        response.get("id").and_then(|v| v.as_str()),
        Some("test-audio-1"),
        "Should match request ID"
    );
    assert!(
        response.get("text").and_then(|v| v.as_str()).is_some(),
        "Should contain text field"
    );

    // Cleanup
    let _ = manager.shutdown().await;
}

#[tokio::test]
async fn ut_4_1_3_multiple_message_sequence() {
    // Test: Should handle multiple message exchanges

    let mut manager = PythonSidecarManager::new();
    manager.start().await.expect("Process should start");
    manager.wait_for_ready().await.expect("Should receive ready");

    // Send 3 ping messages
    for i in 1..=3 {
        let ping = serde_json::json!({
            "type": "ping",
            "id": format!("ping-{}", i)
        });

        manager.send_message(ping).await.expect("Should send ping");

        let response = manager.receive_message().await.expect("Should receive pong");

        assert_eq!(
            response.get("type").and_then(|v| v.as_str()),
            Some("pong"),
            "Should receive pong"
        );
        let expected_id = format!("ping-{}", i);
        assert_eq!(
            response.get("id").and_then(|v| v.as_str()),
            Some(expected_id.as_str()),
            "Should match request ID"
        );
    }

    println!("✅ Successfully exchanged 3 messages");

    // Cleanup
    let _ = manager.shutdown().await;
}

#[tokio::test]
async fn ut_4_1_4_unknown_message_type_error() {
    // Test: Should receive error for unknown message type

    let mut manager = PythonSidecarManager::new();
    manager.start().await.expect("Process should start");
    manager.wait_for_ready().await.expect("Should receive ready");

    // Send unknown message type
    let unknown_msg = serde_json::json!({
        "type": "unknown_type",
        "id": "test-unknown-1"
    });

    manager.send_message(unknown_msg).await.expect("Should send message");

    // Receive error response
    let response = manager.receive_message().await.expect("Should receive error");

    println!("✅ Received error response: {:?}", response);

    assert_eq!(
        response.get("type").and_then(|v| v.as_str()),
        Some("error"),
        "Should receive error type"
    );
    assert!(
        response.get("message").and_then(|v| v.as_str()).is_some(),
        "Should contain error message"
    );

    // Cleanup
    let _ = manager.shutdown().await;
}
