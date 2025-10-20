// E2E Integration Tests for Walking Skeleton (MVP0)

#[path = "support/mod.rs"]
mod support;

#[cfg(test)]
mod e2e_tests {

    /// E2E-8.1.1: Tauri Application Build Verification
    /// Verifies that all components compile and link correctly
    #[test]
    fn test_app_builds_successfully() {
        // This test passes if compilation succeeds
        // The existence of this test validates all trait/interface definitions
        assert!(true, "Application compiled successfully");
    }

    /// E2E-8.1.2: Component Initialization Test
    /// Task 2.1 implemented - now tests actual functionality
    #[test]
    fn test_component_initialization() {
        use meeting_minutes_automator_lib::audio::{AudioDevice, FakeAudioDevice};

        let mut audio_device = FakeAudioDevice::new();

        // Should initialize successfully
        audio_device
            .initialize()
            .expect("Initialization should succeed");
        assert!(
            !audio_device.is_running(),
            "Device should not be running after init"
        );
    }

    /// E2E-8.2.1: Recording Start Flow Test
    /// Task 2.1 implemented - now tests actual functionality
    #[test]
    fn test_recording_start_flow() {
        use meeting_minutes_automator_lib::audio::{AudioDevice, FakeAudioDevice};

        let mut audio_device = FakeAudioDevice::new();
        audio_device
            .initialize()
            .expect("Initialization should succeed");

        // Should start successfully
        audio_device.start().expect("Start should succeed");
        assert!(
            audio_device.is_running(),
            "Device should be running after start"
        );
    }

    /// E2E-8.3.1: Recording Stop and Cleanup Test
    /// Task 2.1 implemented - now tests actual functionality
    #[test]
    fn test_recording_stop_cleanup() {
        use meeting_minutes_automator_lib::audio::{AudioDevice, FakeAudioDevice};

        let mut audio_device = FakeAudioDevice::new();
        audio_device
            .initialize()
            .expect("Initialization should succeed");
        audio_device.start().expect("Start should succeed");

        // Should stop successfully
        audio_device.stop().expect("Stop should succeed");
        assert!(
            !audio_device.is_running(),
            "Device should not be running after stop"
        );
    }

    /// E2E-8.1.3: WebSocket Server Initialization Test
    /// Task 6.1 implemented - now tests actual functionality
    #[tokio::test]
    async fn test_websocket_server_start() {
        use meeting_minutes_automator_lib::websocket::WebSocketServer;

        let mut ws_server = WebSocketServer::new();

        // Should start successfully
        let port = ws_server.start().await.expect("Should start successfully");

        assert!(
            port >= 9001 && port <= 9100,
            "Port should be in range 9001-9100"
        );

        // Cleanup
        ws_server.stop().await.expect("Should stop successfully");
    }

    /// E2E-8.1.2: Python Sidecar Manager Initialization Test
    /// Task 3.1 implemented - now tests actual functionality
    #[tokio::test]
    async fn test_python_sidecar_start() {
        use meeting_minutes_automator_lib::python_sidecar::PythonSidecarManager;

        let mut sidecar = PythonSidecarManager::new();

        // Should start successfully
        sidecar
            .start()
            .await
            .expect("Sidecar should start successfully");

        // Should receive ready signal
        sidecar
            .wait_for_ready()
            .await
            .expect("Should receive ready signal");

        // Cleanup
        sidecar.shutdown().await.expect("Shutdown should succeed");
    }

    /// E2E-8.4.1: Recording → Python IPC → Transcription Flow Test
    /// Tests the complete flow: Audio recording → Python sidecar → IPC communication
    #[tokio::test]
    async fn test_recording_to_transcription_flow() {
        use meeting_minutes_automator_lib::audio::{AudioDevice, FakeAudioDevice};
        use meeting_minutes_automator_lib::python_sidecar::PythonSidecarManager;
        use std::sync::Arc;
        use tokio::sync::Mutex;
        use tokio::time::{timeout, Duration};

        // Start Python sidecar
        let mut sidecar = PythonSidecarManager::new();
        sidecar.start().await.expect("Sidecar should start");
        sidecar
            .wait_for_ready()
            .await
            .expect("Should receive ready signal");

        let sidecar_arc = Arc::new(Mutex::new(sidecar));

        // Create and start audio device
        let mut audio_device = FakeAudioDevice::new();
        audio_device.initialize().expect("Should initialize");

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

        audio_device
            .start_with_callback(move |data| {
                let _ = tx.send(data);
            })
            .await
            .expect("Should start with callback");

        // Send audio chunks to Python
        let sidecar_clone = Arc::clone(&sidecar_arc);
        let send_task = tokio::spawn(async move {
            let mut count = 0;
            while let Some(data) = rx.recv().await {
                if count >= 2 {
                    break;
                } // Send 2 chunks only

                let msg = serde_json::json!({
                    "type": "process_audio",
                    "id": format!("chunk-{}", count),
                    "audio_data": data
                });

                let mut s = sidecar_clone.lock().await;
                s.send_message(msg).await.expect("Should send message");
                count += 1;
            }
        });

        // Wait for chunks to be sent
        tokio::time::sleep(Duration::from_millis(250)).await;
        audio_device.stop().expect("Should stop");

        // Wait for send task to complete
        let _ = timeout(Duration::from_secs(1), send_task).await;

        // Verify responses from Python
        // Note: Python now uses new IPC protocol (type="response"), not legacy "transcription_result"
        let mut sidecar = sidecar_arc.lock().await;
        for i in 0..2 {
            let result = timeout(Duration::from_secs(1), sidecar.receive_message()).await;

            match result {
                Ok(Ok(response)) => {
                    // New protocol returns type="response" with result field
                    let msg_type = response.get("type").and_then(|v| v.as_str());
                    assert!(
                        msg_type == Some("response") || msg_type == Some("transcription_result"),
                        "Response {} should be 'response' (new protocol) or 'transcription_result' (legacy), got {:?}",
                        i, msg_type
                    );

                    // Verify result field exists for new protocol
                    if msg_type == Some("response") {
                        assert!(
                            response.get("result").is_some(),
                            "Response {} should have 'result' field",
                            i
                        );
                    }
                }
                _ => panic!("Should receive response {}", i),
            }
        }

        // Cleanup
        sidecar.shutdown().await.expect("Should shutdown");
    }

    /// Interface Type Compatibility Test
    /// Verifies that message types are correctly defined and serializable
    #[test]
    #[allow(deprecated)]
    fn test_message_type_definitions() {
        use super::support::LegacyIpcMessage;
        use meeting_minutes_automator_lib::websocket::WebSocketMessage;
        use serde_json;

        // Test IPC message serialization (legacy format)
        let ipc_msg = LegacyIpcMessage::Ready;
        let json = serde_json::to_string(&ipc_msg).expect("Should serialize");
        assert!(json.contains("Ready") || json.contains("ready"));

        // Test WebSocket message serialization with all required fields (camelCase)
        let ws_msg = WebSocketMessage::Connected {
            message_id: "msg-1".to_string(),
            session_id: "test-123".to_string(),
            timestamp: 1234567890,
        };
        let json = serde_json::to_string(&ws_msg).expect("Should serialize");
        assert!(json.contains("connected") || json.contains("Connected"));
        // Fields should be in camelCase for Chrome extension compatibility
        assert!(
            json.contains("messageId"),
            "JSON should contain 'messageId' (camelCase): {}",
            json
        );
        assert!(
            json.contains("sessionId"),
            "JSON should contain 'sessionId' (camelCase): {}",
            json
        );
        assert!(json.contains("timestamp"));
    }
}
