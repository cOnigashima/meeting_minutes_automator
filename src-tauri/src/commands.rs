// Tauri Commands
// Walking Skeleton (MVP0) - Recording Commands Implementation
// MVP1 - Audio Device Event Monitoring

use crate::state::AppState;
use crate::websocket::WebSocketMessage;
use crate::audio::AudioDevice;
use crate::audio_device_adapter::AudioDeviceEvent;
use tauri::{State, Manager, AppHandle, Emitter};
use std::sync::Arc;

/// Monitor audio device events and notify UI
/// MVP1 - STT-REQ-004.9/10/11
async fn monitor_audio_events(app: AppHandle) {
    let state = app.state::<AppState>();

    // Take the receiver (can only be done once)
    let rx = match state.take_audio_event_rx() {
        Some(rx) => rx,
        None => {
            eprintln!("[Meeting Minutes] ⚠️ Audio event receiver not available");
            return;
        }
    };

    // Monitor events
    while let Ok(event) = rx.recv() {
        match event {
            AudioDeviceEvent::StreamError(err) => {
                eprintln!("[Meeting Minutes] ❌ Stream error: {}", err);

                // Emit to frontend
                if let Err(e) = app.emit("audio-device-error", serde_json::json!({
                    "type": "stream_error",
                    "message": format!("音声ストリームエラー: {}", err),
                })) {
                    eprintln!("[Meeting Minutes] Failed to emit stream error: {:?}", e);
                }
            }
            AudioDeviceEvent::Stalled { elapsed_ms } => {
                eprintln!("[Meeting Minutes] ⚠️ Audio device stalled: {} ms", elapsed_ms);

                // Emit to frontend
                if let Err(e) = app.emit("audio-device-error", serde_json::json!({
                    "type": "stalled",
                    "message": "音声デバイスが応答しません",
                    "elapsed_ms": elapsed_ms,
                })) {
                    eprintln!("[Meeting Minutes] Failed to emit stalled event: {:?}", e);
                }
            }
            AudioDeviceEvent::DeviceGone { device_id } => {
                eprintln!("[Meeting Minutes] ❌ Device disconnected: {}", device_id);

                // Emit to frontend - STT-REQ-004.10
                if let Err(e) = app.emit("audio-device-error", serde_json::json!({
                    "type": "device_gone",
                    "message": "音声デバイスが切断されました",
                    "device_id": device_id,
                })) {
                    eprintln!("[Meeting Minutes] Failed to emit device gone: {:?}", e);
                }

                // Stop recording automatically
                // TODO: Implement auto-reconnect (STT-REQ-004.11)
                {
                    let state = app.state::<AppState>();
                    let is_recording = state.is_recording.lock().unwrap();
                    if *is_recording {
                        drop(is_recording);
                        eprintln!("[Meeting Minutes] 🛑 Stopping recording due to device disconnection");
                        // Note: Actual stop will be triggered by frontend or timeout
                    }
                }
            }
        }
    }
}

/// Start recording command
/// Starts FakeAudioDevice and processes audio data through Python sidecar
#[tauri::command]
pub async fn start_recording(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    // Check if already recording
    {
        let is_recording = state.is_recording.lock().unwrap();
        if *is_recording {
            return Err("Already recording".to_string());
        }
    }

    // Get references to components
    let audio_device = {
        let device_lock = state.audio_device.lock().unwrap();
        device_lock.clone()
            .ok_or_else(|| "Audio device not initialized".to_string())?
    };

    let python_sidecar = {
        let sidecar_lock = state.python_sidecar.lock().unwrap();
        sidecar_lock.clone()
            .ok_or_else(|| "Python sidecar not initialized".to_string())?
    };

    let websocket_server = {
        let ws_lock = state.websocket_server.lock().unwrap();
        ws_lock.clone()
            .ok_or_else(|| "WebSocket server not initialized".to_string())?
    };

    // Set recording state
    {
        let mut is_recording = state.is_recording.lock().unwrap();
        *is_recording = true;
    }

    // Clone for callback closure
    let python_sidecar_clone = Arc::clone(&python_sidecar);
    let websocket_server_clone = Arc::clone(&websocket_server);

    // Start audio device with callback
    let mut device = audio_device.lock().await;
    device.start_with_callback(move |audio_data| {
        let python_sidecar = Arc::clone(&python_sidecar_clone);
        let websocket_server = Arc::clone(&websocket_server_clone);

        // Spawn async task to handle IPC communication
        tokio::spawn(async move {
            // Send audio data to Python sidecar
            let mut sidecar = python_sidecar.lock().await;

            let message = serde_json::json!({
                "type": "process_audio",
                "id": format!("audio-{}", std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()),
                "audio_data": audio_data,
            });

            if let Err(e) = sidecar.send_message(message).await {
                eprintln!("[Meeting Minutes] Failed to send audio data to Python: {:?}", e);
                return;
            }

            // Receive response from Python
            match sidecar.receive_message().await {
                Ok(response) => {
                    // Extract transcription text
                    if let Some(text) = response.get("text").and_then(|v| v.as_str()) {
                        // Broadcast to WebSocket clients
                        let ws_message = WebSocketMessage::Transcription {
                            message_id: format!("ws-{}", std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis()),
                            session_id: "session-1".to_string(), // TODO: Use actual session ID
                            text: text.to_string(),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64,
                        };

                        let ws_server = websocket_server.lock().await;
                        if let Err(e) = ws_server.broadcast(ws_message).await {
                            eprintln!("[Meeting Minutes] Failed to broadcast transcription: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[Meeting Minutes] Failed to receive Python response: {:?}", e);
                }
            }
        });
    }).await.map_err(|e| e.to_string())?;

    // Start monitoring audio device events - MVP1 STT-REQ-004.9/10/11
    // Note: Event monitoring will start when CoreAudioAdapter is used (not FakeAudioDevice)
    let app_clone = app.clone();
    tokio::spawn(async move {
        monitor_audio_events(app_clone).await;
    });

    println!("[Meeting Minutes] ✅ Recording started");
    Ok("Recording started".to_string())
}

/// Stop recording command
/// Stops FakeAudioDevice
#[tauri::command]
pub async fn stop_recording(state: State<'_, AppState>) -> Result<String, String> {
    // Check if recording
    {
        let is_recording = state.is_recording.lock().unwrap();
        if !*is_recording {
            return Err("Not recording".to_string());
        }
    }

    // Get audio device reference
    let audio_device = {
        let device_lock = state.audio_device.lock().unwrap();
        device_lock.clone()
            .ok_or_else(|| "Audio device not initialized".to_string())?
    };

    // Stop audio device
    let mut device = audio_device.lock().await;
    device.stop().map_err(|e| e.to_string())?;

    // Clear recording state
    {
        let mut is_recording = state.is_recording.lock().unwrap();
        *is_recording = false;
    }

    println!("[Meeting Minutes] ✅ Recording stopped");
    Ok("Recording stopped".to_string())
}
