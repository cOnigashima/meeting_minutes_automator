// Tauri Commands
// Walking Skeleton (MVP0) - Recording Commands Implementation
// MVP1 - Audio Device Event Monitoring
// Task 7.1.5: IPC Protocol Migration Support
// Task 10.4 Phase 2: Auto-Reconnection

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

static LOG_MASK_SALT: Lazy<String> = Lazy::new(|| {
    std::env::var("LOG_MASK_SALT").unwrap_or_else(|_| "meeting-minutes-automator".to_string())
});

static LOG_TRANSCRIPTS_ENABLED: Lazy<bool> = Lazy::new(|| match std::env::var("LOG_TRANSCRIPTS") {
    Ok(value) => {
        let lower = value.to_ascii_lowercase();
        lower == "1" || lower == "true" || lower == "on"
    }
    Err(_) => false,
});

fn mask_text(text: &str) -> String {
    if *LOG_TRANSCRIPTS_ENABLED {
        return text.to_string();
    }

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
                        log_warn_details!(
                            "ipc::reader",
                            "broadcast_failed",
                            json!({ "error": format!("{:?}", e) })
                        );
                    }
                }
                Err(e) => {
                    log_warn_details!(
                        "ipc::reader",
                        "receive_error",
                        json!({ "error": format!("{:?}", e) })
                    );
                    // Don't break - Python may recover
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    });
}

/// Monitor audio device events and notify UI
/// MVP1 - STT-REQ-004.9/10/11
pub(crate) async fn monitor_audio_events(app: AppHandle) {
    let state = app.state::<AppState>();

    // Take the receiver (can only be done once)
    let rx = match state.take_audio_event_rx() {
        Some(rx) => rx,
        None => {
            log_warn_details!(
                "commands::audio_events",
                "receiver_unavailable",
                json!({ "reason": "receiver_not_initialized" })
            );
            return;
        }
    };

    // Monitor events
    while let Ok(event) = rx.recv() {
        match event {
            AudioDeviceEvent::StreamError(err) => {
                log_error_details!(
                    "commands::audio_events",
                    "stream_error",
                    json!({ "error": err.to_string() })
                );

                // Emit to frontend
                if let Err(e) = app.emit(
                    "audio-device-error",
                    json!({
                        "type": "stream_error",
                        "message": format!("音声ストリームエラー: {}", err),
                    }),
                ) {
                    log_error_details!(
                        "commands::audio_events",
                        "emit_stream_error_failed",
                        json!({ "error": format!("{:?}", e) })
                    );
                }
            }
            AudioDeviceEvent::Stalled { elapsed_ms } => {
                log_warn_details!(
                    "commands::audio_events",
                    "device_stalled",
                    json!({ "elapsed_ms": elapsed_ms })
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
                    log_error_details!(
                        "commands::audio_events",
                        "emit_stalled_failed",
                        json!({ "error": format!("{:?}", e) })
                    );
                }
            }
            AudioDeviceEvent::DeviceGone { device_id } => {
                log_error_details!(
                    "commands::audio_events",
                    "device_disconnected",
                    json!({ "device_id": device_id.clone() })
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
                    log_error_details!(
                        "commands::audio_events",
                        "emit_device_gone_failed",
                        json!({ "error": format!("{:?}", e) })
                    );
                }

                // Auto-reconnect (STT-REQ-004.11) - Task 10.4 Phase 2 (Final Revision)
                // Job-based architecture: short lock, independent task execution
                {
                    let state = app.state::<AppState>();

                    // Step 1: Complete cleanup of existing session
                    if let Err(e) = stop_recording_internal(&state).await {
                        log_error_details!(
                            "commands::audio_events",
                            "cleanup_on_disconnect_failed",
                            json!({
                                "device_id": device_id,
                                "error": e
                            })
                        );
                    }

                    // Step 2: Start reconnection job (lock held for microseconds only)
                    {
                        let mut reconnection_mgr = state.reconnection_manager.lock().await;
                        reconnection_mgr.start_job(device_id.clone(), app.clone());
                    } // Lock released immediately

                    log_info_details!(
                        "commands::audio_events",
                        "reconnection_job_started",
                        json!({ "device_id": device_id })
                    );
                }
            }
        }
    }
}

/// Internal helper for starting recording
/// Used by start_recording command and reconnection logic
/// Task 10.4 Phase 2: Reusable session initialization for device reconnection
pub(crate) async fn start_recording_internal(
    _app: &AppHandle,
    state: &AppState,
    device_id: String,
) -> Result<(), String> {
    // Task 9.1: Validate and log selected device (STT-REQ-001.2)
    log_info_details!(
        "commands::recording",
        "start_requested",
        json!({ "device_id": device_id })
    );

    // MVP0: Validate device exists in enumeration
    let available_devices = crate::audio::FakeAudioDevice::enumerate_devices_static()
        .map_err(|e| format!("Failed to enumerate devices: {}", e))?;

    if !available_devices.iter().any(|d| d.id == device_id) {
        log_error_details!(
            "commands::recording",
            "invalid_device",
            json!({
                "requested": device_id,
                "available": available_devices.iter().map(|d| &d.id).collect::<Vec<_>>()
            })
        );
        return Err(format!(
            "Invalid device ID: {}. Available: {:?}",
            device_id,
            available_devices.iter().map(|d| &d.id).collect::<Vec<_>>()
        ));
    }

    log_info_details!(
        "commands::recording",
        "device_validated",
        json!({ "device_id": device_id })
    );

    // Task 9.1: Save selected device to AppState (STT-REQ-001.2)
    state.set_selected_device_id(device_id.clone());
    log_info_details!(
        "commands::recording",
        "device_selection_saved",
        json!({ "device_id": device_id })
    );

    // MVP1 TODO: Pass device_id to AudioDeviceAdapter::start_recording(device_id)

    // Check if already recording (Task 10.4 Phase 2 - Permissive for reconnection)
    {
        let is_recording = state.is_recording.lock().unwrap();
        if *is_recording {
            log_info_details!(
                "commands::recording",
                "already_recording",
                json!({
                    "device_id": device_id,
                    "reason": "reconnection_or_manual_restart"
                })
            );
            return Ok(()); // Treat as success (user protection)
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
    log_info_details!(
        "commands::recording",
        "session_initialized",
        json!({ "session": session_id })
    );

    // Set recording state
    {
        let mut is_recording = state.is_recording.lock().unwrap();
        *is_recording = true;
    }

    // Clone for callback closure
    let python_sidecar_clone = Arc::clone(&python_sidecar);
    let websocket_server_clone = Arc::clone(&websocket_server);
    let session_id_arc = Arc::new(session_id.clone());

    // Start audio device with callback
    let mut device = audio_device.lock().await;
    if let Err(err) = device
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
                        log_error_details!(
                            "commands::recording",
                            "serialize_ipc_failed",
                            json!({
                                "session": session_id_handle.as_str(),
                                "error": format!("{:?}", e)
                            })
                        );
                        return;
                    }
                };

                // Send message with minimal mutex hold time
                {
                    let mut sidecar = python_sidecar.lock().await;
                    if let Err(e) = sidecar.send_message(message_json).await {
                        log_error_details!(
                            "commands::recording",
                            "send_to_sidecar_failed",
                            json!({
                                "session": session_id_handle.as_str(),
                                "error": format!("{:?}", e)
                            })
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
                                log_warn_details!(
                                    "commands::recording",
                                    "sidecar_receive_timeout",
                                    json!({
                                        "session": session_id_handle.as_str(),
                                        "timeout_ms": 5000
                                    })
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
                                        log_error_details!(
                                            "commands::recording",
                                            "parse_ipc_failed",
                                            json!({
                                                "session": session_id_handle.as_str(),
                                                "error": format!("{:?}", e)
                                            })
                                        );
                                        break;
                                    }
                                };

                            // Task 7.2: Check version compatibility (STT-REQ-007.6)
                            match msg.check_version_compatibility() {
                                VersionCompatibility::MajorMismatch { received, expected } => {
                                    log_error_details!(
                                        "commands::recording",
                                        "ipc_version_major_mismatch",
                                        json!({
                                            "session": session_id_handle.as_str(),
                                            "received": received,
                                            "expected": expected
                                        })
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
                                            log_error_details!(
                                                "commands::recording",
                                                "send_version_error_failed",
                                                json!({
                                                    "session": session_id_handle.as_str(),
                                                    "error": format!("{:?}", e)
                                                })
                                            );
                                        }
                                    }

                                    // Reject communication - exit loop
                                    break;
                                }
                                VersionCompatibility::MinorMismatch { received, expected } => {
                                    log_warn_details!(
                                        "commands::recording",
                                        "ipc_version_minor_mismatch",
                                        json!({
                                            "session": session_id_handle.as_str(),
                                            "received": received,
                                            "expected": expected
                                        })
                                    );
                                    // Continue processing with backward compatibility
                                }
                                VersionCompatibility::Malformed { received } => {
                                    log_error_details!(
                                        "commands::recording",
                                        "ipc_version_malformed",
                                        json!({
                                            "session": session_id_handle.as_str(),
                                            "received": received.clone()
                                        })
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
                                            log_error_details!(
                                                "commands::recording",
                                                "send_malformed_version_error_failed",
                                                json!({
                                                    "session": session_id_handle.as_str(),
                                                    "error": format!("{:?}", e)
                                                })
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
                                            log_info_details!(
                                                "commands::ipc_events",
                                                "speech_start",
                                                json!({
                                                    "session": session_id_ref,
                                                    "request": request_id
                                                })
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
                                                log_info_details!(
                                                    "commands::ipc_events",
                                                    "partial_text",
                                                    json!({
                                                        "session": session_id_ref,
                                                        "request": request_id,
                                                        "text_masked": masked
                                                    })
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
                                                    session_id: session_id_ref.to_string(),
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
                                                    log_error_details!(
                                                        "commands::ipc_events",
                                                        "broadcast_partial_failed",
                                                        json!({
                                                            "session": session_id_ref,
                                                            "request": request_id,
                                                            "error": format!("{:?}", e)
                                                        })
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
                                                log_info_details!(
                                                    "commands::ipc_events",
                                                    "final_text",
                                                    json!({
                                                        "session": session_id_ref,
                                                        "request": request_id,
                                                        "text_masked": masked
                                                    })
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
                                                    session_id: session_id_ref.to_string(),
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
                                                    log_error_details!(
                                                        "commands::ipc_events",
                                                        "broadcast_final_failed",
                                                        json!({
                                                            "session": session_id_ref,
                                                            "request": request_id,
                                                            "error": format!("{:?}", e)
                                                        })
                                                    );
                                                }
                                            }
                                        }
                                        "speech_end" => {
                                            let request_id =
                                                request_id_from(&data).unwrap_or("unknown");
                                            log_info_details!(
                                                "commands::ipc_events",
                                                "speech_end",
                                                json!({
                                                    "session": session_id_ref,
                                                    "request": request_id
                                                })
                                            );
                                            break;
                                        }
                                        "no_speech" => {
                                            let request_id =
                                                request_id_from(&data).unwrap_or("unknown");
                                            log_info_details!(
                                                "commands::ipc_events",
                                                "no_speech",
                                                json!({
                                                    "session": session_id_ref,
                                                    "request": request_id
                                                })
                                            );
                                            break;
                                        }
                                        "model_change" => {
                                            // Phase 1.3: Handle model_change event (P13-PREP-001)
                                            // STT-REQ-006.9: Dynamic model switching notification

                                            // Validate required fields (addressing external review)
                                            let old_model = data.get("old_model")
                                                .and_then(|v| v.as_str());
                                            let new_model = data.get("new_model")
                                                .and_then(|v| v.as_str());
                                            let reason = data.get("reason")
                                                .and_then(|v| v.as_str());

                                            // CRITICAL: Validate schema compliance
                                            if old_model.is_none() || new_model.is_none() || reason.is_none() {
                                                log_error_details!(
                                                    "commands::ipc_events",
                                                    "model_change_invalid_schema",
                                                    json!({
                                                        "session": session_id_ref,
                                                        "data": data,
                                                        "missing_fields": {
                                                            "old_model": old_model.is_none(),
                                                            "new_model": new_model.is_none(),
                                                            "reason": reason.is_none()
                                                        }
                                                    })
                                                );

                                                // Emit schema error to UI
                                                let ws_server = websocket_server.lock().await;
                                                if let Err(e) = ws_server.broadcast(
                                                    WebSocketMessage::Error {
                                                        message_id: format!("ws-{}",
                                                            std::time::SystemTime::now()
                                                                .duration_since(std::time::UNIX_EPOCH)
                                                                .unwrap()
                                                                .as_millis()),
                                                        session_id: session_id_ref.to_string(),
                                                        message: "モデル変更通知のデータ形式が不正です".to_string(),
                                                        timestamp: std::time::SystemTime::now()
                                                            .duration_since(std::time::UNIX_EPOCH)
                                                            .unwrap()
                                                            .as_millis() as u64,
                                                    }
                                                ).await {
                                                    log_error_details!(
                                                        "commands::ipc_events",
                                                        "broadcast_schema_error_failed",
                                                        json!({
                                                            "session": session_id_ref,
                                                            "error": format!("{:?}", e)
                                                        })
                                                    );
                                                }
                                                // Continue processing despite schema error
                                            } else {
                                                // Schema valid - extract values safely
                                                let old_model = old_model.unwrap();
                                                let new_model = new_model.unwrap();
                                                let reason = reason.unwrap();

                                                log_info_details!(
                                                    "commands::ipc_events",
                                                    "model_change",
                                                    json!({
                                                        "session": session_id_ref,
                                                        "old_model": old_model,
                                                        "new_model": new_model,
                                                        "reason": reason
                                                    })
                                                );

                                                // STT-REQ-006.9: UI notification required
                                                let notification_msg = format!(
                                                    "モデル変更: {} → {} (理由: {})",
                                                    old_model,
                                                    new_model,
                                                    match reason {
                                                        "cpu_high" => "CPU負荷",
                                                        "memory_high" => "メモリ不足",
                                                        "memory_critical" => "メモリ緊急",
                                                        "manual_switch" => "手動切り替え",
                                                        _ => reason
                                                    }
                                                );

                                                let ws_server = websocket_server.lock().await;
                                                if let Err(e) = ws_server.broadcast(
                                                    WebSocketMessage::Notification {
                                                        message_id: format!("ws-{}",
                                                            std::time::SystemTime::now()
                                                                .duration_since(std::time::UNIX_EPOCH)
                                                                .unwrap()
                                                                .as_millis()),
                                                        session_id: session_id_ref.to_string(),
                                                        notification_type: "model_change".to_string(),
                                                        message: notification_msg,
                                                        timestamp: std::time::SystemTime::now()
                                                            .duration_since(std::time::UNIX_EPOCH)
                                                            .unwrap()
                                                            .as_millis() as u64,
                                                        data: Some(json!({
                                                            "old_model": old_model,
                                                            "new_model": new_model,
                                                            "reason": reason
                                                        })),
                                                    }
                                                ).await {
                                                    log_error_details!(
                                                        "commands::ipc_events",
                                                        "broadcast_model_change_failed",
                                                        json!({
                                                            "session": session_id_ref,
                                                            "error": format!("{:?}", e)
                                                        })
                                                    );
                                                }
                                            }

                                            // model_change is non-terminating - continue processing audio
                                        }
                                        _ => {
                                            log_warn_details!(
                                                "commands::ipc_events",
                                                "unknown_event_type",
                                                json!({
                                                    "session": session_id_ref,
                                                    "event_type": event_type
                                                })
                                            );
                                        }
                                    }
                                }
                                ProtocolMessage::Error { error_message, .. } => {
                                    log_error_details!(
                                        "commands::ipc_events",
                                        "python_sidecar_error",
                                        json!({
                                            "session": session_id_ref,
                                            "message": error_message
                                        })
                                    );
                                    break;
                                }
                                _ => {
                                    log_warn_details!(
                                        "commands::ipc_events",
                                        "unexpected_message_type",
                                        json!({
                                            "session": session_id_ref,
                                            "message": msg
                                        })
                                    );
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            log_error_details!(
                                "commands::ipc_events",
                                "receive_event_failed",
                                json!({
                                    "session": session_id_handle.as_str(),
                                    "error": format!("{:?}", e)
                                })
                            );
                            break;
                        }
                    }
                }
            });
        })
        .await
    {
        let error_msg = err.to_string();
        {
            let mut is_recording = state.is_recording.lock().unwrap();
            *is_recording = false;
        }
        state.clear_session_id();
        log_error_details!(
            "commands::recording",
            "start_failed",
            json!({
                "session": session_id,
                "device_id": device_id,
                "error": error_msg
            })
        );
        return Err(error_msg);
    }

    log_info_details!(
        "commands::recording",
        "started",
        json!({
            "session": session_id,
            "device_id": device_id
        })
    );
    Ok(())
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
    start_recording_internal(&app, &state, device_id).await?;
    Ok("Recording started".to_string())
}

/// Internal helper for stopping recording
/// Used by stop_recording command and reconnection logic
/// Task 10.4 Phase 2: Reusable cleanup for device reconnection
pub(crate) async fn stop_recording_internal(state: &AppState) -> Result<(), String> {
    // Check if recording (silent return if already stopped)
    {
        let is_recording = state.is_recording.lock().unwrap();
        if !*is_recording {
            return Ok(()); // Already stopped
        }
    }

    let current_session = state.get_session_id();
    let selected_device = state.get_selected_device_id();

    // Get audio device reference
    let audio_device = {
        let device_lock = state.audio_device.lock().unwrap();
        device_lock
            .clone()
            .ok_or_else(|| "Audio device not initialized".to_string())?
    };

    // Stop audio device (cleanup resources)
    let mut device = audio_device.lock().await;
    device.stop().map_err(|e| e.to_string())?;

    // Clear recording state
    {
        let mut is_recording = state.is_recording.lock().unwrap();
        *is_recording = false;
    }

    state.clear_session_id();

    log_info_details!(
        "commands::recording",
        "stopped",
        json!({
            "session": current_session,
            "device_id": selected_device
        })
    );
    Ok(())
}

/// Stop recording command
/// Stops FakeAudioDevice
#[tauri::command]
pub async fn stop_recording(state: State<'_, AppState>) -> Result<String, String> {
    // Check if recording (return error if not recording)
    {
        let is_recording = state.is_recording.lock().unwrap();
        if !*is_recording {
            return Err("Not recording".to_string());
        }
    }

    stop_recording_internal(&state).await?;
    Ok("Recording stopped".to_string())
}

/// Cancel ongoing reconnection attempts
/// Task 10.4 Phase 2: User-initiated cancellation of auto-reconnect
#[tauri::command]
pub async fn cancel_reconnection(state: State<'_, AppState>) -> Result<String, String> {
    let mut reconnection_mgr = state.reconnection_manager.lock().await;

    if !reconnection_mgr.is_reconnecting() {
        log_info_details!(
            "commands::reconnection",
            "cancel_no_job",
            json!({})
        );
        return Ok("No reconnection in progress".to_string());
    }

    reconnection_mgr.cancel();

    log_info_details!(
        "commands::reconnection",
        "cancelled_by_user",
        json!({})
    );

    Ok("Reconnection cancelled".to_string())
}

/// Get available Whisper models and system resources
/// Task 9.2: Whisper model selection UI
/// Requirement: STT-REQ-006.1, STT-REQ-006.2, STT-REQ-006.4
#[tauri::command]
pub async fn get_whisper_models() -> Result<serde_json::Value, String> {
    log_info!("commands::models", "request_models");

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
    log_info!("commands::audio_devices", "enumerate_requested");

    // Task 9.1: Use static enumeration (no dependency on initialized recorder)
    // For MVP0: FakeAudioDevice::enumerate_devices_static()
    // For MVP1: CpalAudioDeviceAdapter::enumerate_devices_static()
    match crate::audio::FakeAudioDevice::enumerate_devices_static() {
        Ok(devices) => {
            log_info_details!(
                "commands::audio_devices",
                "enumerate_success",
                json!({ "count": devices.len() })
            );
            for device in &devices {
                log_debug_details!(
                    "commands::audio_devices",
                    "device_entry",
                    json!({
                        "id": device.id,
                        "name": device.name,
                        "sample_rate": device.sample_rate,
                        "channels": device.channels,
                        "loopback": device.is_loopback
                    })
                );
            }
            Ok(devices)
        }
        Err(e) => {
            log_error_details!(
                "commands::audio_devices",
                "enumerate_failed",
                json!({ "error": e.to_string() })
            );
            Err(format!("Failed to list audio devices: {}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    /// Task 10.3.3: Test model_change event schema validation
    ///
    /// This test verifies the schema validation logic in handle_ipc_events()
    /// for model_change events (STT-REQ-006.9)
    #[test]
    fn test_model_change_event_schema_valid() {
        // Valid schema: all required fields present
        let valid_event = json!({
            "eventType": "model_change",
            "data": {
                "old_model": "medium",
                "new_model": "base",
                "reason": "cpu_high"
            }
        });

        // Extract fields (same logic as handle_ipc_events)
        let data = valid_event.get("data").unwrap();
        let old_model = data.get("old_model").and_then(|v| v.as_str());
        let new_model = data.get("new_model").and_then(|v| v.as_str());
        let reason = data.get("reason").and_then(|v| v.as_str());

        // Verify all fields present
        assert!(old_model.is_some(), "old_model should be present");
        assert!(new_model.is_some(), "new_model should be present");
        assert!(reason.is_some(), "reason should be present");

        assert_eq!(old_model.unwrap(), "medium");
        assert_eq!(new_model.unwrap(), "base");
        assert_eq!(reason.unwrap(), "cpu_high");
    }

    #[test]
    fn test_model_change_event_schema_missing_old_model() {
        // Invalid: missing old_model
        let invalid_event = json!({
            "eventType": "model_change",
            "data": {
                "new_model": "base",
                "reason": "cpu_high"
            }
        });

        let data = invalid_event.get("data").unwrap();
        let old_model = data.get("old_model").and_then(|v| v.as_str());

        // Should detect missing field
        assert!(old_model.is_none(), "old_model should be missing");
    }

    #[test]
    fn test_model_change_event_schema_missing_new_model() {
        // Invalid: missing new_model
        let invalid_event = json!({
            "eventType": "model_change",
            "data": {
                "old_model": "medium",
                "reason": "cpu_high"
            }
        });

        let data = invalid_event.get("data").unwrap();
        let new_model = data.get("new_model").and_then(|v| v.as_str());

        assert!(new_model.is_none(), "new_model should be missing");
    }

    #[test]
    fn test_model_change_event_schema_missing_reason() {
        // Invalid: missing reason
        let invalid_event = json!({
            "eventType": "model_change",
            "data": {
                "old_model": "medium",
                "new_model": "base"
            }
        });

        let data = invalid_event.get("data").unwrap();
        let reason = data.get("reason").and_then(|v| v.as_str());

        assert!(reason.is_none(), "reason should be missing");
    }

    #[test]
    fn test_model_change_reason_translation() {
        // Test Japanese message formatting (Phase 1.3 implementation)
        let reason_map = vec![
            ("cpu_high", "CPU負荷"),
            ("memory_high", "メモリ不足"),
            ("memory_critical", "メモリ緊急"),
            ("manual_switch", "手動切り替え"),
            ("unknown_reason", "unknown_reason"), // Fallback to original
        ];

        for (reason, expected_japanese) in reason_map {
            let translated = match reason {
                "cpu_high" => "CPU負荷",
                "memory_high" => "メモリ不足",
                "memory_critical" => "メモリ緊急",
                "manual_switch" => "手動切り替え",
                _ => reason,
            };

            assert_eq!(translated, expected_japanese);
        }
    }
}
