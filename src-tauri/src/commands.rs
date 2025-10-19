// Tauri Commands
// Walking Skeleton (MVP0) - Recording Commands Implementation
// MVP1 - Audio Device Event Monitoring
// Task 7.1.5: IPC Protocol Migration Support

use crate::audio::AudioDevice;
use crate::audio_device_adapter::AudioDeviceEvent;
use crate::ipc_protocol::{IpcMessage as ProtocolMessage, VersionCompatibility, PROTOCOL_VERSION};
use crate::state::AppState;
use crate::websocket::WebSocketMessage;
use once_cell::sync::Lazy;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::broadcast;
use uuid::Uuid;

static LOG_MASK_SALT: Lazy<String> =
    Lazy::new(|| std::env::var("LOG_MASK_SALT").unwrap_or_else(|_| Uuid::new_v4().to_string()));

fn mask_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(LOG_MASK_SALT.as_bytes());
    hasher.update(text.as_bytes());
    let digest = hasher.finalize();
    let hash_prefix = hex::encode(&digest[..8]);
    let char_len = text.chars().count();
    format!("len={} hash={}", char_len, hash_prefix)
}

fn request_id_from(data: &serde_json::Value) -> Option<&str> {
    data.get("requestId").and_then(|v| v.as_str())
}

/// Helper: Extract extended fields from IPC event data (STT-REQ-008.1)
/// Used by both partial_text and final_text branches to avoid code duplication
fn extract_extended_fields(
    data: &serde_json::Map<String, serde_json::Value>,
) -> (Option<f64>, Option<String>, Option<u64>) {
    let confidence = data.get("confidence").and_then(|v| v.as_f64());
    let language = data
        .get("language")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let processing_time_ms = data.get("processing_time_ms").and_then(|v| v.as_u64());
    (confidence, language, processing_time_ms)
}

/// Global IPC event reader task (spawned once at startup)
/// Solves deadlock: single reader distributes events to all listeners via broadcast channel
/// Related: STT-REQ-007 (Event Stream Protocol deadlock fix)
async fn start_ipc_reader_task(
    python_sidecar: Arc<tokio::sync::Mutex<crate::python_sidecar::PythonSidecarManager>>,
    event_tx: broadcast::Sender<serde_json::Value>,
) {
    tokio::spawn(async move {
        loop {
            // Acquire mutex ONLY for receive, then immediately drop
            let event_result = {
                let mut sidecar = python_sidecar.lock().await;
                sidecar.receive_message().await
            }; // Mutex dropped here

            match event_result {
                Ok(event) => {
                    // Broadcast to all subscribers (non-blocking)
                    if let Err(e) = event_tx.send(event.clone()) {
                        log_warn!("ipc::reader", "broadcast_failed", format!("{:?}", e));
                    }
                }
                Err(e) => {
                    log_warn!("ipc::reader", "receive_error", format!("{:?}", e));
                    // Don't break - Python may recover
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    });
}

/// Monitor audio device events and notify UI
/// MVP1 - STT-REQ-004.9/10/11
async fn monitor_audio_events(app: AppHandle) {
    let state = app.state::<AppState>();

    // Take the receiver (can only be done once)
    let rx = match state.take_audio_event_rx() {
        Some(rx) => rx,
        None => {
            log_warn!("commands::audio_events", "receiver_unavailable", "");
            return;
        }
    };

    // Monitor events
    while let Ok(event) = rx.recv() {
        match event {
            AudioDeviceEvent::StreamError(err) => {
                log_error!("commands::audio_events", "stream_error", err.to_string());

                // Emit to frontend
                if let Err(e) = app.emit(
                    "audio-device-error",
                    json!({
                        "type": "stream_error",
                        "message": format!("音声ストリームエラー: {}", err),
                    }),
                ) {
                    log_error!(
                        "commands::audio_events",
                        "emit_stream_error_failed",
                        format!("{:?}", e)
                    );
                }
            }
            AudioDeviceEvent::Stalled { elapsed_ms } => {
                log_warn!(
                    "commands::audio_events",
                    "device_stalled",
                    format!("elapsed_ms={}", elapsed_ms)
                );

                // Emit to frontend
                if let Err(e) = app.emit(
                    "audio-device-error",
                    json!({
                        "type": "stalled",
                        "message": "音声デバイスが応答しません",
                        "elapsed_ms": elapsed_ms,
                    }),
                ) {
                    log_error!(
                        "commands::audio_events",
                        "emit_stalled_failed",
                        format!("{:?}", e)
                    );
                }
            }
            AudioDeviceEvent::DeviceGone { device_id } => {
                log_error!(
                    "commands::audio_events",
                    "device_disconnected",
                    device_id.clone()
                );

                // Emit to frontend - STT-REQ-004.10
                if let Err(e) = app.emit(
                    "audio-device-error",
                    json!({
                        "type": "device_gone",
                        "message": "音声デバイスが切断されました",
                        "device_id": device_id,
                    }),
                ) {
                    log_error!(
                        "commands::audio_events",
                        "emit_device_gone_failed",
                        format!("{:?}", e)
                    );
                }

                // Stop recording automatically
                // TODO: Implement auto-reconnect (STT-REQ-004.11)
                {
                    let state = app.state::<AppState>();
                    let is_recording = state.is_recording.lock().unwrap();
                    if *is_recording {
                        drop(is_recording);
                        log_warn!("commands::audio_events", "auto_stop_on_disconnect", "");
                        // Note: Actual stop will be triggered by frontend or timeout
                    }
                }
            }
        }
    }
}

/// Start recording command
/// Starts FakeAudioDevice and processes audio data through Python sidecar
/// Task 9.1: Accept device_id to honor user's device selection (STT-REQ-001.2)
#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: String,
) -> Result<String, String> {
    // Task 9.1: Validate and log selected device (STT-REQ-001.2)
    log_info!(
        "commands::recording",
        "start_requested",
        format!("device_id={}", device_id)
    );

    // MVP0: Validate device exists in enumeration
    let available_devices = crate::audio::FakeAudioDevice::enumerate_devices_static()
        .map_err(|e| format!("Failed to enumerate devices: {}", e))?;

    if !available_devices.iter().any(|d| d.id == device_id) {
        log_error!(
            "commands::recording",
            "invalid_device",
            format!(
                "requested={}, available={:?}",
                device_id,
                available_devices.iter().map(|d| &d.id).collect::<Vec<_>>()
            )
        );
        return Err(format!(
            "Invalid device ID: {}. Available: {:?}",
            device_id,
            available_devices.iter().map(|d| &d.id).collect::<Vec<_>>()
        ));
    }

    log_info!("commands::recording", "device_validated", device_id.clone());

    // Task 9.1: Save selected device to AppState (STT-REQ-001.2)
    state.set_selected_device_id(device_id.clone());
    log_info!(
        "commands::recording",
        "device_selection_saved",
        device_id.clone()
    );

    // MVP1 TODO: Pass device_id to AudioDeviceAdapter::start_recording(device_id)

    // Check if already recording
    {
        let is_recording = state.is_recording.lock().unwrap();
        if *is_recording {
            log_warn!("commands::recording", "start_rejected_already_running", "");
            return Err("Already recording".to_string());
        }
    }

    // Get references to components
    let audio_device = {
        let device_lock = state.audio_device.lock().unwrap();
        device_lock
            .clone()
            .ok_or_else(|| "Audio device not initialized".to_string())?
    };

    let python_sidecar = {
        let sidecar_lock = state.python_sidecar.lock().unwrap();
        sidecar_lock
            .clone()
            .ok_or_else(|| "Python sidecar not initialized".to_string())?
    };

    let websocket_server = {
        let ws_lock = state.websocket_server.lock().unwrap();
        ws_lock
            .clone()
            .ok_or_else(|| "WebSocket server not initialized".to_string())?
    };

    let session_id = Uuid::new_v4().to_string();
    state.set_session_id(session_id.clone());
    log_info!(
        "commands::recording",
        "session_initialized",
        format!("session_id={}", session_id)
    );

    // Set recording state
    {
        let mut is_recording = state.is_recording.lock().unwrap();
        *is_recording = true;
    }

    // Clone for callback closure
    let python_sidecar_clone = Arc::clone(&python_sidecar);
    let websocket_server_clone = Arc::clone(&websocket_server);
    let session_id_arc = Arc::new(session_id);

    // Start audio device with callback
    let mut device = audio_device.lock().await;
    device
        .start_with_callback(move |audio_data| {
            let python_sidecar = Arc::clone(&python_sidecar_clone);
            let websocket_server = Arc::clone(&websocket_server_clone);
            let session_id_handle = Arc::clone(&session_id_arc);

            // Spawn async task to handle IPC communication
            tokio::spawn(async move {
                // CRITICAL FIX: Narrow mutex scope to prevent deadlock on long utterances
                // Previous bug: Held mutex from send_message through entire receive loop,
                // blocking subsequent audio chunks from being sent. This prevented Python
                // from receiving later frames needed to detect speech_end, causing permanent deadlock.
                // Fix: Release mutex immediately after send, re-acquire for each receive.
                // Requirement: STT-REQ-007 (non-blocking event stream)

                // Task 7.1.6: Use event stream protocol (STT-REQ-007.3)
                // Python側がprocess_audio_streamに対応済み（2025-10-13）
                let message = ProtocolMessage::Request {
                    id: format!(
                        "audio-{}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis()
                    ),
                    version: PROTOCOL_VERSION.to_string(),
                    method: "process_audio_stream".to_string(),
                    params: serde_json::json!({ "audio_data": audio_data }),
                };

                let message_json = match serde_json::to_value(&message) {
                    Ok(json) => json,
                    Err(e) => {
                        log_error!(
                            "commands::recording",
                            "serialize_ipc_failed",
                            format!("{:?}", e)
                        );
                        return;
                    }
                };

                // Send message with minimal mutex hold time
                {
                    let mut sidecar = python_sidecar.lock().await;
                    if let Err(e) = sidecar.send_message(message_json).await {
                        log_error!(
                            "commands::recording",
                            "send_to_sidecar_failed",
                            format!("{:?}", e)
                        );
                        return;
                    }
                    // Mutex dropped here - other audio chunks can now send frames
                }

                // Task 7.1.6: Receive multiple events (speech_start, partial_text, final_text, speech_end)
                // Loop until speech_end or error
                //
                // REMAINING ISSUE: MutexGuard is held across .await even within {}
                // This still blocks other chunks from sending. Proper fix requires
                // dedicated background receiver task (see ADR-XXX).
                // Current mitigation: 5-second timeout to prevent permanent deadlock.
                loop {
                    // WARNING: This still holds mutex during await, despite {} block
                    let response = {
                        let mut sidecar = python_sidecar.lock().await;

                        // Add timeout to prevent permanent deadlock
                        match tokio::time::timeout(
                            std::time::Duration::from_secs(5),
                            sidecar.receive_message(),
                        )
                        .await
                        {
                            Ok(result) => result,
                            Err(_) => {
                                log_warn!(
                                    "commands::recording",
                                    "sidecar_receive_timeout",
                                    "duration_ms=5000"
                                );
                                break;
                            }
                        }
                        // Mutex dropped here
                    };

                    match response {
                        Ok(response) => {
                            // Parse IPC message
                            let msg =
                                match serde_json::from_value::<ProtocolMessage>(response.clone()) {
                                    Ok(m) => m,
                                    Err(e) => {
                                        log_error!(
                                            "commands::recording",
                                            "parse_ipc_failed",
                                            format!("{:?}", e)
                                        );
                                        break;
                                    }
                                };

                            // Task 7.2: Check version compatibility (STT-REQ-007.6)
                            match msg.check_version_compatibility() {
                                VersionCompatibility::MajorMismatch { received, expected } => {
                                    log_error!(
                                        "commands::recording",
                                        "ipc_version_major_mismatch",
                                        format!("received={}, expected={}", received, expected)
                                    );

                                    // P0 FIX: Send error response to Python before breaking
                                    // STT-REQ-007.6: "エラー応答を返し、通信を拒否"
                                    let error_response = ProtocolMessage::Error {
                                        id: msg.id().to_string(),
                                        version: PROTOCOL_VERSION.to_string(),
                                        error_code: "VERSION_MISMATCH_MAJOR".to_string(),
                                        error_message: format!(
                                            "Major version mismatch: received={}, expected={}",
                                            received, expected
                                        ),
                                        recoverable: false,
                                    };

                                    if let Ok(json) = serde_json::to_value(&error_response) {
                                        let mut sidecar = python_sidecar.lock().await;
                                        if let Err(e) = sidecar.send_message(json).await {
                                            log_error!(
                                                "commands::recording",
                                                "send_version_error_failed",
                                                format!("{:?}", e)
                                            );
                                        }
                                    }

                                    // Reject communication - exit loop
                                    break;
                                }
                                VersionCompatibility::MinorMismatch { received, expected } => {
                                    log_warn!(
                                        "commands::recording",
                                        "ipc_version_minor_mismatch",
                                        format!("received={}, expected={}", received, expected)
                                    );
                                    // Continue processing with backward compatibility
                                }
                                VersionCompatibility::Malformed { received } => {
                                    log_error!(
                                        "commands::recording",
                                        "ipc_version_malformed",
                                        received.clone()
                                    );

                                    // P0 FIX: Send error response to Python before breaking
                                    // STT-REQ-007.6: "エラー応答を返し、通信を拒否"
                                    let error_response = ProtocolMessage::Error {
                                        id: msg.id().to_string(),
                                        version: PROTOCOL_VERSION.to_string(),
                                        error_code: "VERSION_MALFORMED".to_string(),
                                        error_message: format!(
                                            "Malformed version string: {}",
                                            received
                                        ),
                                        recoverable: false,
                                    };

                                    if let Ok(json) = serde_json::to_value(&error_response) {
                                        let mut sidecar = python_sidecar.lock().await;
                                        if let Err(e) = sidecar.send_message(json).await {
                                            log_error!(
                                                "commands::recording",
                                                "send_malformed_version_error_failed",
                                                format!("{:?}", e)
                                            );
                                        }
                                    }

                                    // Reject communication - exit loop
                                    break;
                                }
                                VersionCompatibility::Compatible => {
                                    // Version is compatible - continue normally
                                }
                            }

                            let session_id_ref = session_id_handle.as_ref();
                            match msg {
                                ProtocolMessage::Event {
                                    event_type, data, ..
                                } => {
                                    match event_type.as_str() {
                                        "speech_start" => {
                                            let request_id =
                                                request_id_from(&data).unwrap_or("unknown");
                                            log_info!(
                                                "commands::ipc_events",
                                                "speech_start",
                                                format!(
                                                    "session={} request={}",
                                                    session_id_ref, request_id
                                                )
                                            );
                                            // TODO: Emit to frontend if needed
                                        }
                                        "partial_text" => {
                                            let request_id =
                                                request_id_from(&data).unwrap_or("unknown");
                                            if let Some(text) =
                                                data.get("text").and_then(|v| v.as_str())
                                            {
                                                let masked = mask_text(text);
                                                log_info!(
                                                    "commands::ipc_events",
                                                    "partial_text",
                                                    format!(
                                                        "session={} request={} {}",
                                                        session_id_ref, request_id, masked
                                                    )
                                                );

                                                let (confidence, language, processing_time_ms) =
                                                    if let Some(obj) = data.as_object() {
                                                        extract_extended_fields(obj)
                                                    } else {
                                                        (None, None, None)
                                                    };

                                                let ws_message = WebSocketMessage::Transcription {
                                                    message_id: format!(
                                                        "ws-{}",
                                                        std::time::SystemTime::now()
                                                            .duration_since(std::time::UNIX_EPOCH)
                                                            .unwrap()
                                                            .as_millis()
                                                    ),
                                                    session_id: "session-1".to_string(),
                                                    text: text.to_string(),
                                                    timestamp: std::time::SystemTime::now()
                                                        .duration_since(std::time::UNIX_EPOCH)
                                                        .unwrap()
                                                        .as_millis()
                                                        as u64,
                                                    is_partial: Some(true),
                                                    confidence,
                                                    language,
                                                    processing_time_ms,
                                                };

                                                let ws_server = websocket_server.lock().await;
                                                if let Err(e) =
                                                    ws_server.broadcast(ws_message).await
                                                {
                                                    log_error!(
                                                        "commands::ipc_events",
                                                        "broadcast_partial_failed",
                                                        format!("{:?}", e)
                                                    );
                                                }
                                            }
                                        }
                                        "final_text" => {
                                            let request_id =
                                                request_id_from(&data).unwrap_or("unknown");
                                            if let Some(text) =
                                                data.get("text").and_then(|v| v.as_str())
                                            {
                                                let masked = mask_text(text);
                                                log_info!(
                                                    "commands::ipc_events",
                                                    "final_text",
                                                    format!(
                                                        "session={} request={} {}",
                                                        session_id_ref, request_id, masked
                                                    )
                                                );

                                                let (confidence, language, processing_time_ms) =
                                                    if let Some(obj) = data.as_object() {
                                                        extract_extended_fields(obj)
                                                    } else {
                                                        (None, None, None)
                                                    };

                                                let ws_message = WebSocketMessage::Transcription {
                                                    message_id: format!(
                                                        "ws-{}",
                                                        std::time::SystemTime::now()
                                                            .duration_since(std::time::UNIX_EPOCH)
                                                            .unwrap()
                                                            .as_millis()
                                                    ),
                                                    session_id: "session-1".to_string(),
                                                    text: text.to_string(),
                                                    timestamp: std::time::SystemTime::now()
                                                        .duration_since(std::time::UNIX_EPOCH)
                                                        .unwrap()
                                                        .as_millis()
                                                        as u64,
                                                    is_partial: Some(false),
                                                    confidence,
                                                    language,
                                                    processing_time_ms,
                                                };

                                                let ws_server = websocket_server.lock().await;
                                                if let Err(e) =
                                                    ws_server.broadcast(ws_message).await
                                                {
                                                    log_error!(
                                                        "commands::ipc_events",
                                                        "broadcast_final_failed",
                                                        format!("{:?}", e)
                                                    );
                                                }
                                            }
                                        }
                                        "speech_end" => {
                                            let request_id =
                                                request_id_from(&data).unwrap_or("unknown");
                                            log_info!(
                                                "commands::ipc_events",
                                                "speech_end",
                                                format!(
                                                    "session={} request={}",
                                                    session_id_ref, request_id
                                                )
                                            );
                                            break;
                                        }
                                        "no_speech" => {
                                            let request_id =
                                                request_id_from(&data).unwrap_or("unknown");
                                            log_info!(
                                                "commands::ipc_events",
                                                "no_speech",
                                                format!(
                                                    "session={} request={}",
                                                    session_id_ref, request_id
                                                )
                                            );
                                            break;
                                        }
                                        _ => {
                                            log_warn!(
                                                "commands::ipc_events",
                                                "unknown_event_type",
                                                event_type
                                            );
                                        }
                                    }
                                }
                                ProtocolMessage::Error { error_message, .. } => {
                                    log_error!(
                                        "commands::ipc_events",
                                        "python_sidecar_error",
                                        format!(
                                            "session={} message={}",
                                            session_id_ref, error_message
                                        )
                                    );
                                    break;
                                }
                                _ => {
                                    log_warn!(
                                        "commands::ipc_events",
                                        "unexpected_message_type",
                                        format!("session={} message={:?}", session_id_ref, msg)
                                    );
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            log_error!(
                                "commands::ipc_events",
                                "receive_event_failed",
                                format!("session={} message={:?}", session_id_handle.as_ref(), e)
                            );
                            break;
                        }
                    }
                }
            });
        })
        .await
        .map_err(|e| e.to_string())?;

    // Start monitoring audio device events - MVP1 STT-REQ-004.9/10/11
    // Note: Event monitoring will start when CoreAudioAdapter is used (not FakeAudioDevice)
    let app_clone = app.clone();
    tokio::spawn(async move {
        monitor_audio_events(app_clone).await;
    });

    log_info!("commands::recording", "started", "");
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
        device_lock
            .clone()
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

    state.clear_session_id();

    log_info!("commands::recording", "stopped", "");
    Ok("Recording stopped".to_string())
}

/// Get available Whisper models and system resources
/// Task 9.2: Whisper model selection UI
/// Requirement: STT-REQ-006.1, STT-REQ-006.2, STT-REQ-006.4
#[tauri::command]
pub async fn get_whisper_models() -> Result<serde_json::Value, String> {
    log_info!("commands::models", "request_models", "");

    // Task 9.2: Available models (STT-REQ-006.2)
    let models = vec!["tiny", "base", "small", "medium", "large-v3"];

    // Task 9.2: Get system resources (MVP0: static values, MVP1: actual detection)
    let system_resources = serde_json::json!({
        "cpu_cores": num_cpus::get(),
        "total_memory_gb": 8,  // MVP0: static, TODO: actual detection
        "gpu_available": false,  // MVP0: static, TODO: actual detection
        "gpu_memory_gb": 0,
    });

    // Task 9.2: Calculate recommended model based on STT-REQ-006.2
    let recommended_model = calculate_recommended_model(&system_resources);

    Ok(serde_json::json!({
        "available_models": models,
        "system_resources": system_resources,
        "recommended_model": recommended_model,
    }))
}

/// Calculate recommended Whisper model based on system resources
/// Implements STT-REQ-006.2 model selection rules
fn calculate_recommended_model(resources: &serde_json::Value) -> String {
    let memory_gb = resources["total_memory_gb"].as_f64().unwrap_or(4.0);
    let gpu_available = resources["gpu_available"].as_bool().unwrap_or(false);
    let gpu_memory_gb = resources["gpu_memory_gb"].as_f64().unwrap_or(0.0);

    if gpu_available && memory_gb >= 8.0 && gpu_memory_gb >= 10.0 {
        "large-v3".to_string()
    } else if gpu_available && memory_gb >= 4.0 && gpu_memory_gb >= 5.0 {
        "medium".to_string()
    } else if memory_gb >= 4.0 {
        "small".to_string()
    } else if memory_gb >= 2.0 {
        "base".to_string()
    } else {
        "tiny".to_string()
    }
}

/// List available audio input devices
/// Task 9.1: Audio device selection UI
/// Requirement: STT-REQ-001.1, STT-REQ-001.2
///
/// IMPORTANT: Decoupled from recorder instance to allow enumeration before recording starts.
/// This matches the real device adapter pattern (CoreAudio/WASAPI/ALSA perform static host queries).
#[tauri::command]
pub async fn list_audio_devices(
    _state: State<'_, AppState>,
) -> Result<Vec<crate::audio_device_adapter::AudioDeviceInfo>, String> {
    log_info!("commands::audio_devices", "enumerate_requested", "");

    // Task 9.1: Use static enumeration (no dependency on initialized recorder)
    // For MVP0: FakeAudioDevice::enumerate_devices_static()
    // For MVP1: CpalAudioDeviceAdapter::enumerate_devices_static()
    match crate::audio::FakeAudioDevice::enumerate_devices_static() {
        Ok(devices) => {
            log_info!(
                "commands::audio_devices",
                "enumerate_success",
                format!("count={}", devices.len())
            );
            for device in &devices {
                log_debug!(
                    "commands::audio_devices",
                    "device_entry",
                    format!(
                        "id={}, name={}, sample_rate={}, channels={}, loopback={}",
                        device.id,
                        device.name,
                        device.sample_rate,
                        device.channels,
                        device.is_loopback
                    )
                );
            }
            Ok(devices)
        }
        Err(e) => {
            log_error!("commands::audio_devices", "enumerate_failed", e.to_string());
            Err(format!("Failed to list audio devices: {}", e))
        }
    }
}
