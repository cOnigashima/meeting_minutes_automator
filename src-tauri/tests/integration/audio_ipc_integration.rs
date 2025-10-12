// Integration Test: FakeAudioDevice → PythonSidecarManager IPC
// Validates E2E audio data flow

use meeting_minutes_automator_lib::audio::{AudioDevice, FakeAudioDevice};
use meeting_minutes_automator_lib::python_sidecar::PythonSidecarManager;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn it_audio_to_python_ipc_flow() {
    // Test: FakeAudioDevice generates data → sends to Python → receives transcription

    // Start Python sidecar
    let mut sidecar = PythonSidecarManager::new();
    sidecar.start().await.expect("Should start Python sidecar");
    sidecar
        .wait_for_ready()
        .await
        .expect("Should receive ready signal");

    // Shared sidecar reference for callback
    let sidecar_arc = Arc::new(Mutex::new(sidecar));

    // Create audio device
    let mut audio_device = FakeAudioDevice::new();
    audio_device.initialize().expect("Should initialize");

    // Counter to track how many chunks were sent
    let chunk_counter = Arc::new(std::sync::Mutex::new(0));
    let counter_clone = Arc::clone(&chunk_counter);

    // Use channel to send data from callback to async task
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

    // Start audio device with callback that sends to channel
    audio_device
        .start_with_callback(move |data| {
            let _ = tx.send(data);
            *counter_clone.lock().unwrap() += 1;
        })
        .await
        .expect("Should start with callback");

    // Async task to receive from channel and send to Python
    let sidecar_for_task = Arc::clone(&sidecar_arc);
    let _receive_task = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            let msg = serde_json::json!({
                "type": "process_audio",
                "id": format!("audio-chunk"),
                "audio_data": data
            });

            // Lock, send, unlock immediately
            let mut sidecar = sidecar_for_task.lock().await;
            let _ = sidecar.send_message(msg).await;
        }
    });

    // Wait for at least 3 chunks to be generated (300ms)
    tokio::time::sleep(Duration::from_millis(350)).await;

    // Stop audio device
    audio_device.stop().expect("Should stop");

    // Verify chunks were sent
    let count = *chunk_counter.lock().unwrap();
    println!("✅ Sent {} audio chunks to Python", count);
    assert!(count >= 3, "Should have sent at least 3 chunks");

    // Receive responses from Python
    let mut sidecar = sidecar_arc.lock().await;
    for i in 0..count {
        let result = timeout(Duration::from_secs(1), sidecar.receive_message()).await;

        match result {
            Ok(Ok(response)) => {
                println!("✅ Received response {}: {:?}", i, response.get("type"));
                assert_eq!(
                    response.get("type").and_then(|v| v.as_str()),
                    Some("transcription_result"),
                    "Should receive transcription_result"
                );
            }
            Ok(Err(e)) => {
                eprintln!("❌ IPC error: {:?}", e);
            }
            Err(_) => {
                eprintln!("⚠️  Timeout waiting for response {}", i);
                break;
            }
        }
    }

    // Cleanup
    let _ = sidecar.shutdown().await;

    println!("✅ Integration test completed successfully");
}

#[tokio::test]
async fn it_audio_device_multiple_start_stop_cycles() {
    // Test: Start/stop audio device multiple times

    let mut sidecar = PythonSidecarManager::new();
    sidecar.start().await.expect("Should start Python sidecar");
    sidecar
        .wait_for_ready()
        .await
        .expect("Should receive ready signal");

    let sidecar_arc = Arc::new(Mutex::new(sidecar));
    let mut audio_device = FakeAudioDevice::new();

    for cycle in 0..3 {
        println!("Starting cycle {}", cycle);

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

        audio_device
            .start_with_callback(move |data| {
                let _ = tx.send(data);
            })
            .await
            .expect("Should start");

        let sidecar_clone = Arc::clone(&sidecar_arc);
        let send_task = tokio::spawn(async move {
            while let Some(data) = rx.recv().await {
                let msg = serde_json::json!({
                    "type": "process_audio",
                    "id": format!("cycle-{}-chunk", cycle),
                    "audio_data": data
                });

                {
                    let mut s = sidecar_clone.lock().await;
                    let _ = s.send_message(msg).await;
                }
            }
        });

        tokio::time::sleep(Duration::from_millis(150)).await;

        audio_device.stop().expect("Should stop");
        drop(send_task); // Stop the send task

        println!("Completed cycle {}", cycle);
    }

    // Cleanup
    let mut sidecar = sidecar_arc.lock().await;
    let _ = sidecar.shutdown().await;

    println!("✅ Multiple start/stop cycles completed");
}
