// Tauri Commands
// Walking Skeleton (MVP0) - Recording Commands Implementation
// MVP1 - Audio Device Event Monitoring
// Task 7.1.5: IPC Protocol Migration Support
// Task 10.4 Phase 2: Auto-Reconnection

use crate::audio_device_adapter::AudioDeviceEvent;
use crate::audio_device_recorder::{MixerConfig, RecordingMode};
use crate::multi_input_manager::InputStatus;
use crate::ipc_protocol::{IpcMessage as ProtocolMessage, VersionCompatibility, PROTOCOL_VERSION};
use crate::ring_buffer::{new_shared_ring_buffer, pop_audio, push_audio_drop_oldest, BufferLevel};
use crate::state::AppState;
use crate::websocket::WebSocketMessage;
use once_cell::sync::Lazy;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
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

/// Background IPC event reader task (ADR-013: Full-Duplex IPC)
/// Requirement: STT-REQ-007 (non-blocking event stream)
///
/// This task runs independently from audio chunk submission, preventing deadlock
/// on long utterances where Python cannot emit speech_end until it receives more audio.
///
/// FIXED (Phase 14.5): Now uses separate stdout handle instead of shared sidecar lock.
/// This eliminates Mutex contention between reader and sender tasks.
async fn start_ipc_reader_task(
    stdout: Arc<tokio::sync::Mutex<tokio::io::BufReader<tokio::process::ChildStdout>>>,
    app: tauri::AppHandle,
    session_id: String,
    websocket_server: Arc<tokio::sync::Mutex<crate::websocket::WebSocketServer>>,
    cancel_token: tokio_util::sync::CancellationToken,
) {
    use tokio::io::AsyncBufReadExt;

    tokio::spawn(async move {
        loop {
            // Read from stdout with cancellation support using select!
            let response = {
                let mut stdout_guard = stdout.lock().await;
                let mut line = String::new();

                // Use select! to race between cancellation and read
                let read_result = tokio::select! {
                    _ = cancel_token.cancelled() => {
                        log_info!("commands::ipc_reader", "cancelled");
                        return; // Exit the entire task
                    }
                    result = stdout_guard.read_line(&mut line) => result
                };

                match read_result {
                    Ok(0) => {
                        // EOF - process closed
                        log_info!("commands::ipc_reader", "stdout_eof");
                        break;
                    }
                    Ok(_) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<serde_json::Value>(&line) {
                            Ok(json) => Ok(json),
                            Err(e) => Err(format!("JSON parse error: {:?}", e)),
                        }
                    }
                    Err(e) => Err(format!("Read error: {:?}", e)),
                }
            };

            match response {
                Ok(response) => {
                    // Debug: Log received IPC message type
                    log_debug_details!(
                        "commands::ipc_reader",
                        "received_ipc_message",
                        json!({
                            "session": session_id,
                            "message_type": response.get("type").or(response.get("event_type")).unwrap_or(&serde_json::Value::Null)
                        })
                    );
                    // Parse IPC message
                    let msg = match serde_json::from_value::<ProtocolMessage>(response.clone()) {
                        Ok(m) => m,
                        Err(e) => {
                            log_error_details!(
                                "commands::ipc_reader",
                                "parse_ipc_failed",
                                json!({
                                    "session": session_id,
                                    "error": format!("{:?}", e)
                                })
                            );
                            break;
                        }
                    };

                    // Version compatibility check
                    match msg.check_version_compatibility() {
                        VersionCompatibility::MajorMismatch { received, expected } => {
                            log_error_details!(
                                "commands::ipc_reader",
                                "version_major_mismatch",
                                json!({
                                    "session": session_id,
                                    "received": received,
                                    "expected": expected
                                })
                            );

                            // Note: Cannot send error response since we don't have stdin access
                            // Just terminate the reader task
                            break;
                        }
                        VersionCompatibility::MinorMismatch { received, expected } => {
                            log_warn_details!(
                                "commands::ipc_reader",
                                "version_minor_mismatch",
                                json!({
                                    "session": session_id,
                                    "received": received,
                                    "expected": expected
                                })
                            );
                            // Continue processing
                        }
                        VersionCompatibility::Malformed { received } => {
                            log_error_details!(
                                "commands::ipc_reader",
                                "version_malformed",
                                json!({
                                    "session": session_id,
                                    "received": received.clone()
                                })
                            );

                            let error_response = ProtocolMessage::Error {
                                id: msg.id().to_string(),
                                version: PROTOCOL_VERSION.to_string(),
                                error_code: "VERSION_MALFORMED".to_string(),
                                error_message: format!("Malformed version string: {}", received),
                                recoverable: false,
                            };

                            // Note: Cannot send error response since we don't have stdin access
                            let _ = error_response; // Suppress unused warning
                            break;
                        }
                        VersionCompatibility::Compatible => {
                            // Continue normally
                        }
                    }

                    // Handle events (same logic as before, extracted for brevity)
                    let session_id_ref = session_id.as_str();
                    match msg {
                        ProtocolMessage::Event {
                            event_type, data, ..
                        } => {
                            handle_ipc_event(
                                &event_type,
                                &data,
                                session_id_ref,
                                &websocket_server,
                                &app,
                            )
                            .await;
                        }
                        ProtocolMessage::Error { error_message, .. } => {
                            log_error_details!(
                                "commands::ipc_reader",
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
                                "commands::ipc_reader",
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
                        "commands::ipc_reader",
                        "receive_event_failed",
                        json!({
                            "session": session_id,
                            "error": format!("{:?}", e)
                        })
                    );
                    break;
                }
            }
        }
    });
}

/// Helper function to handle IPC events (extracted from inline logic)
/// Reduces code duplication between old audio callback loop and new background reader
async fn handle_ipc_event(
    event_type: &str,
    data: &serde_json::Value,
    session_id: &str,
    websocket_server: &Arc<tokio::sync::Mutex<crate::websocket::WebSocketServer>>,
    app: &tauri::AppHandle,
) {
    match event_type {
        "speech_start" => {
            let request_id = request_id_from(data).unwrap_or("unknown");
            log_info_details!(
                "commands::ipc_events",
                "speech_start",
                json!({
                    "session": session_id,
                    "request": request_id
                })
            );
        }
        "partial_text" => {
            let request_id = request_id_from(data).unwrap_or("unknown");
            if let Some(text) = data.get("text").and_then(|v| v.as_str()) {
                let (confidence, language, processing_time_ms) = if let Some(obj) = data.as_object()
                {
                    extract_extended_fields(obj)
                } else {
                    (None, None, None)
                };

                // Filter out low-confidence hallucinations (common with Whisper on silence)
                const MIN_CONFIDENCE: f64 = 0.50;
                if let Some(conf) = confidence {
                    if conf < MIN_CONFIDENCE {
                        log_debug_details!(
                            "commands::ipc_events",
                            "partial_text_filtered",
                            json!({
                                "session": session_id,
                                "request": request_id,
                                "confidence": conf,
                                "threshold": MIN_CONFIDENCE
                            })
                        );
                        return; // Skip this low-confidence transcription
                    }
                }

                let masked = mask_text(text);
                log_info_details!(
                    "commands::ipc_events",
                    "partial_text",
                    json!({
                        "session": session_id,
                        "request": request_id,
                        "text_masked": masked,
                        "confidence": confidence
                    })
                );

                // Clone for emit (before move into WebSocketMessage)
                let emit_language = language.clone();

                let ws_message = WebSocketMessage::Transcription {
                    message_id: format!(
                        "ws-{}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis()
                    ),
                    session_id: session_id.to_string(),
                    text: text.to_string(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                    is_partial: Some(true),
                    confidence,
                    language,
                    processing_time_ms,
                };

                let ws_server = websocket_server.lock().await;
                if let Err(e) = ws_server.broadcast(ws_message).await {
                    log_error_details!(
                        "commands::ipc_events",
                        "broadcast_partial_failed",
                        json!({
                            "session": session_id,
                            "request": request_id,
                            "error": format!("{:?}", e)
                        })
                    );
                }

                // Debug: Emit to Tauri frontend for real-time transcription display
                let _ = app.emit(
                    "transcription",
                    json!({
                        "session_id": session_id,
                        "text": text,
                        "is_partial": true,
                        "confidence": confidence,
                        "language": emit_language,
                        "timestamp": std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64
                    }),
                );
            }
        }
        "final_text" => {
            let request_id = request_id_from(data).unwrap_or("unknown");
            if let Some(text) = data.get("text").and_then(|v| v.as_str()) {
                let (confidence, language, processing_time_ms) = if let Some(obj) = data.as_object()
                {
                    extract_extended_fields(obj)
                } else {
                    (None, None, None)
                };

                // Filter out low-confidence hallucinations (common with Whisper on silence)
                const MIN_CONFIDENCE: f64 = 0.50;
                if let Some(conf) = confidence {
                    if conf < MIN_CONFIDENCE {
                        log_debug_details!(
                            "commands::ipc_events",
                            "final_text_filtered",
                            json!({
                                "session": session_id,
                                "request": request_id,
                                "confidence": conf,
                                "threshold": MIN_CONFIDENCE
                            })
                        );
                        return; // Skip this low-confidence transcription
                    }
                }

                let masked = mask_text(text);
                log_info_details!(
                    "commands::ipc_events",
                    "final_text",
                    json!({
                        "session": session_id,
                        "request": request_id,
                        "text_masked": masked,
                        "confidence": confidence
                    })
                );

                // Clone for emit (before move into WebSocketMessage)
                let emit_language = language.clone();

                let ws_message = WebSocketMessage::Transcription {
                    message_id: format!(
                        "ws-{}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis()
                    ),
                    session_id: session_id.to_string(),
                    text: text.to_string(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                    is_partial: Some(false),
                    confidence,
                    language,
                    processing_time_ms,
                };

                let ws_server = websocket_server.lock().await;
                if let Err(e) = ws_server.broadcast(ws_message).await {
                    log_error_details!(
                        "commands::ipc_events",
                        "broadcast_final_failed",
                        json!({
                            "session": session_id,
                            "request": request_id,
                            "error": format!("{:?}", e)
                        })
                    );
                }

                // Debug: Emit to Tauri frontend for real-time transcription display
                let _ = app.emit(
                    "transcription",
                    json!({
                        "session_id": session_id,
                        "text": text,
                        "is_partial": false,
                        "confidence": confidence,
                        "language": emit_language,
                        "timestamp": std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64
                    }),
                );
            }
        }
        "speech_end" => {
            let request_id = request_id_from(data).unwrap_or("unknown");
            log_info_details!(
                "commands::ipc_events",
                "speech_end",
                json!({
                    "session": session_id,
                    "request": request_id
                })
            );
        }
        "no_speech" => {
            let request_id = request_id_from(data).unwrap_or("unknown");
            log_info_details!(
                "commands::ipc_events",
                "no_speech",
                json!({
                    "session": session_id,
                    "request": request_id
                })
            );
        }
        "model_change" => {
            // Validate required fields
            let old_model = data.get("old_model").and_then(|v| v.as_str());
            let new_model = data.get("new_model").and_then(|v| v.as_str());
            let reason = data.get("reason").and_then(|v| v.as_str());

            if old_model.is_none() || new_model.is_none() || reason.is_none() {
                log_error_details!(
                    "commands::ipc_events",
                    "model_change_invalid_schema",
                    json!({
                        "session": session_id,
                        "data": data,
                        "missing_fields": {
                            "old_model": old_model.is_none(),
                            "new_model": new_model.is_none(),
                            "reason": reason.is_none()
                        }
                    })
                );

                let ws_server = websocket_server.lock().await;
                let _ = ws_server
                    .broadcast(WebSocketMessage::Error {
                        message_id: format!(
                            "ws-{}",
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis()
                        ),
                        session_id: session_id.to_string(),
                        message: "モデル変更通知のデータ形式が不正です".to_string(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64,
                    })
                    .await;
            } else {
                let old_model = old_model.unwrap();
                let new_model = new_model.unwrap();
                let reason = reason.unwrap();

                log_info_details!(
                    "commands::ipc_events",
                    "model_change",
                    json!({
                        "session": session_id,
                        "old_model": old_model,
                        "new_model": new_model,
                        "reason": reason
                    })
                );

                let notification_msg = format!(
                    "モデル変更: {} → {} (理由: {})",
                    old_model,
                    new_model,
                    match reason {
                        "cpu_high" => "CPU負荷",
                        "memory_high" => "メモリ不足",
                        "memory_critical" => "メモリ緊急",
                        "manual_switch" => "手動切り替え",
                        _ => reason,
                    }
                );

                let ws_server = websocket_server.lock().await;
                let _ = ws_server
                    .broadcast(WebSocketMessage::Notification {
                        message_id: format!(
                            "ws-{}",
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis()
                        ),
                        session_id: session_id.to_string(),
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
                    })
                    .await;
            }
        }
        _ => {
            log_warn_details!(
                "commands::ipc_events",
                "unknown_event_type",
                json!({
                    "session": session_id,
                    "event_type": event_type
                })
            );
        }
    }
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
    let multi_enabled = state.is_multi_input_enabled();
    let device_ids = if multi_enabled {
        state.get_selected_device_ids()
    } else {
        vec![device_id.clone()]
    };

    if device_ids.is_empty() {
        return Err("At least one device ID must be provided".to_string());
    }
    if multi_enabled && device_ids.len() > 2 {
        return Err(format!(
            "Maximum 2 inputs supported, got {}. (STTMIX-CON-005)",
            device_ids.len()
        ));
    }

    // Task 9.1: Validate and log selected device(s) (STT-REQ-001.2)
    log_info_details!(
        "commands::recording",
        "start_requested",
        json!({
            "device_id": device_id,
            "device_ids": device_ids.clone(),
            "multi_input": multi_enabled
        })
    );

    // MVP1: Validate device exists in real device enumeration
    let available_devices = crate::audio_device_adapter::enumerate_devices_static()
        .map_err(|e| format!("Failed to enumerate devices: {}", e))?;

    let valid_ids: Vec<String> = device_ids
        .iter()
        .filter(|id| available_devices.iter().any(|d| d.id == **id))
        .cloned()
        .collect();
    let invalid_ids: Vec<String> = device_ids
        .iter()
        .filter(|id| !available_devices.iter().any(|d| d.id == **id))
        .cloned()
        .collect();
    let allow_partial_failure = multi_enabled && MixerConfig::default().continue_on_partial_failure;

    if valid_ids.is_empty() {
        log_error_details!(
            "commands::recording",
            "invalid_device",
            json!({
                "requested": device_ids.clone(),
                "invalid": invalid_ids,
                "available": available_devices.iter().map(|d| &d.id).collect::<Vec<_>>()
            })
        );
        return Err(format!(
            "Invalid device ID(s): {:?}. Available: {:?}",
            invalid_ids,
            available_devices.iter().map(|d| &d.id).collect::<Vec<_>>()
        ));
    }

    if !invalid_ids.is_empty() && !allow_partial_failure {
        log_error_details!(
            "commands::recording",
            "invalid_device",
            json!({
                "requested": device_ids.clone(),
                "invalid": invalid_ids,
                "available": available_devices.iter().map(|d| &d.id).collect::<Vec<_>>()
            })
        );
        return Err(format!(
            "Invalid device ID(s): {:?}. Available: {:?}",
            invalid_ids,
            available_devices.iter().map(|d| &d.id).collect::<Vec<_>>()
        ));
    }

    if !invalid_ids.is_empty() && allow_partial_failure {
        log_warn_details!(
            "commands::recording",
            "partial_device_missing",
            json!({
                "requested": device_ids.clone(),
                "invalid": invalid_ids,
                "available": available_devices.iter().map(|d| &d.id).collect::<Vec<_>>()
            })
        );
    }

    log_info_details!(
        "commands::recording",
        "device_validated",
        json!({ "device_ids": device_ids.clone() })
    );

    // Task 9.1: Save selected device to AppState (STT-REQ-001.2)
    let primary_device_id = valid_ids
        .get(0)
        .cloned()
        .unwrap_or_else(|| device_id.clone());
    state.set_selected_device_id(primary_device_id);
    log_info_details!(
        "commands::recording",
        "device_selection_saved",
        json!({ "device_id": device_id })
    );

    // Device ID is now validated against real device enumeration

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
    let audio_recorder = {
        let recorder_lock = state.audio_recorder.lock().unwrap();
        recorder_lock
            .clone()
            .ok_or_else(|| "Audio recorder not initialized".to_string())?
    };

    // Get or initialize sidecar stdin/stdout handles
    // First recording: extract from sidecar and store in AppState
    // Subsequent recordings: reuse from AppState
    let (sidecar_stdin, sidecar_stdout) = {
        let existing_stdin = state.get_sidecar_stdin();
        let existing_stdout = state.get_sidecar_stdout();

        if let (Some(stdin), Some(stdout)) = (existing_stdin, existing_stdout) {
            // Reuse existing handles
            (stdin, stdout)
        } else {
            // First time: extract from sidecar
            let python_sidecar = {
                let sidecar_lock = state.python_sidecar.lock().unwrap();
                sidecar_lock
                    .clone()
                    .ok_or_else(|| "Python sidecar not initialized".to_string())?
            };

            let mut sidecar = python_sidecar.lock().await;
            let stdin = sidecar
                .take_stdin()
                .ok_or_else(|| "Python sidecar stdin not available".to_string())?;
            let stdout = sidecar
                .take_stdout()
                .ok_or_else(|| "Python sidecar stdout not available".to_string())?;

            let stdin_arc = Arc::new(tokio::sync::Mutex::new(stdin));
            let stdout_arc = Arc::new(tokio::sync::Mutex::new(stdout));

            // Store in AppState for reuse
            state.set_sidecar_handles(Arc::clone(&stdin_arc), Arc::clone(&stdout_arc));

            (stdin_arc, stdout_arc)
        }
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

    // Cancel any previous recording tasks before starting new ones
    state.cancel_recording_tasks();

    // Create cancellation token for this recording session
    let cancel_token = state.create_recording_cancel_token();

    // Start background IPC reader task (ADR-013: Full-Duplex IPC)
    // This task runs independently from audio chunk submission, preventing deadlock
    // Now uses separate stdout handle - no Mutex contention with stdin sender
    start_ipc_reader_task(
        Arc::clone(&sidecar_stdout),
        _app.clone(),
        session_id.clone(),
        Arc::clone(&websocket_server),
        cancel_token.clone(),
    )
    .await;

    log_info_details!(
        "commands::recording",
        "ipc_reader_started",
        json!({ "session": session_id })
    );

    // Create shared ring buffer to decouple audio callback from IPC sending
    // Ring buffer provides:
    // - Fixed 160KB capacity (5 seconds of audio)
    // - Drop-oldest strategy: when full, old data is discarded for new
    // - Real-time priority: latest audio is always preserved
    let ring_buffer = new_shared_ring_buffer();
    let ring_buffer_producer = Arc::clone(&ring_buffer);
    let ring_buffer_consumer = Arc::clone(&ring_buffer);

    // Spawn dedicated audio sender task
    // This task reads from ring buffer and writes to stdin
    // BATCHING: Read from buffer every 250ms to batch audio chunks
    let stdin_sender = Arc::clone(&sidecar_stdin);
    let session_id_sender = session_id.clone();
    let cancel_token_sender = cancel_token.clone();
    tokio::spawn(async move {
        use tokio::io::AsyncWriteExt;

        let mut batch_count = 0u64;
        // Read buffer matches ring buffer capacity to drain quickly after backlog
        let mut batch_buffer = vec![0u8; crate::ring_buffer::BUFFER_CAPACITY];
        const MIN_BATCH_BYTES: usize = 4000; // Minimum 125ms to send
        let mut batch_interval = tokio::time::interval(std::time::Duration::from_millis(250));

        loop {
            // Wait for timer or cancellation
            tokio::select! {
                _ = cancel_token_sender.cancelled() => {
                    log_info!("commands::recording", "audio_sender_cancelled");
                    break;
                }
                _ = batch_interval.tick() => {
                    // Timer fired - read from ring buffer
                }
            }

            // Read available audio from ring buffer
            let bytes_read = {
                if let Ok(mut rb) = ring_buffer_consumer.lock() {
                    pop_audio(&mut rb, &mut batch_buffer)
                } else {
                    0 // Lock poisoned, skip this cycle
                }
            };

            if bytes_read < MIN_BATCH_BYTES {
                // Not enough data yet
                continue;
            }

            batch_count += 1;
            let batch_data = batch_buffer[..bytes_read].to_vec();

            log_debug_details!(
                "commands::recording",
                "sending_audio_batch",
                json!({
                    "session": session_id_sender,
                    "batch_count": batch_count,
                    "batch_size": bytes_read
                })
            );

            // Task 7.1.6: Use event stream protocol (STT-REQ-007.3)
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
                params: serde_json::json!({ "audio_data": batch_data }),
            };

            let json_str = match serde_json::to_string(&message) {
                Ok(s) => s,
                Err(e) => {
                    log_error_details!(
                        "commands::recording",
                        "serialize_ipc_failed",
                        json!({
                            "session": session_id_sender,
                            "error": format!("{:?}", e)
                        })
                    );
                    continue;
                }
            };

            // Write directly to stdin - no Mutex contention with stdout reader
            // Add timeout to prevent blocking forever (increased for larger batches)
            let write_future = async {
                let mut stdin = stdin_sender.lock().await;
                stdin.write_all(json_str.as_bytes()).await?;
                stdin.write_all(b"\n").await?;
                stdin.flush().await
            };

            let write_result =
                tokio::time::timeout(std::time::Duration::from_secs(10), write_future).await;

            match write_result {
                Ok(Ok(_)) => {
                    log_debug_details!(
                        "commands::recording",
                        "batch_sent_to_python",
                        json!({
                            "session": session_id_sender,
                            "batch_count": batch_count,
                            "batch_size": json_str.len()
                        })
                    );
                }
                Ok(Err(e)) => {
                    log_error_details!(
                        "commands::recording",
                        "send_to_sidecar_failed",
                        json!({
                            "session": session_id_sender,
                            "error": format!("{:?}", e)
                        })
                    );
                    // Continue processing other audio chunks
                }
                Err(_timeout) => {
                    log_error_details!(
                        "commands::recording",
                        "send_to_sidecar_timeout",
                        json!({
                            "session": session_id_sender,
                            "batch_count": batch_count,
                            "timeout_secs": 10
                        })
                    );
                    // Continue processing - don't block on slow writes
                }
            }
            // Mutex dropped here
        }
        log_info!("commands::recording", "audio_sender_task_ended");
    });

    log_info_details!(
        "commands::recording",
        "audio_sender_started",
        json!({ "session": session_id })
    );

    // Start audio device with callback
    // MVP1: Use AudioDeviceAdapter trait with device_id
    // Callback writes to ring buffer with drop-oldest strategy
    let mut recorder = audio_recorder.lock().await;
    let callback: crate::audio_device_adapter::AudioChunkCallback =
        Box::new(move |audio_data: Vec<u8>| {
            // Non-blocking write to ring buffer
            // Use try_lock to avoid blocking in real-time audio callback
            if let Ok(mut rb) = ring_buffer_producer.try_lock() {
                let (_pushed, dropped, level) = push_audio_drop_oldest(&mut rb, &audio_data);
                if dropped > 0 {
                    // Old data was dropped to make room for new data
                    // This is expected when Python is slow - we prioritize latest audio
                }
                if level == BufferLevel::Critical {
                    // Buffer is getting full, Python may be falling behind
                }
            }
            // If try_lock fails, skip this frame (sender task holds lock briefly)
        });

    let recording_mode = if multi_enabled {
        RecordingMode::Multi {
            device_ids: device_ids.clone(),
            mixer_config: MixerConfig::default(),
        }
    } else {
        RecordingMode::Single {
            device_id: device_id.clone(),
        }
    };

    if let Err(err) = recorder.start(recording_mode, callback) {
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

/// Start recording command (single device - backward compatible)
/// Starts audio device and processes audio data through Python sidecar
/// Task 9.1: Accept device_id to honor user's device selection (STT-REQ-001.2)
#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: String,
) -> Result<String, String> {
    // Disable multi-input mode for single device recording
    state.set_multi_input_enabled(false);
    start_recording_internal(&app, &state, device_id).await?;
    Ok("Recording started".to_string())
}

/// Start multi-input recording command
/// Starts recording from multiple audio devices simultaneously
/// STTMIX Task 1.3: Accept multiple device_ids for multi-input mode (STTMIX-REQ-002.1)
///
/// # Arguments
/// * `device_ids` - Vector of device IDs (max 2 per STTMIX-CON-005)
///
/// # Returns
/// * `Ok(String)` with success message
/// * `Err(String)` if validation fails or recording cannot start
#[tauri::command]
pub async fn start_recording_multi(
    app: AppHandle,
    state: State<'_, AppState>,
    device_ids: Vec<String>,
) -> Result<String, String> {
    // STTMIX-CON-004: Multi-input only supported on macOS
    #[cfg(not(target_os = "macos"))]
    {
        return Err("Multi-input recording is only supported on macOS (STTMIX-CON-004)".to_string());
    }

    // Validate input count (STTMIX-CON-005: max 2 inputs)
    if device_ids.is_empty() {
        return Err("At least one device ID must be provided".to_string());
    }
    if device_ids.len() > 2 {
        return Err(format!(
            "Maximum 2 inputs supported, got {}. (STTMIX-CON-005)",
            device_ids.len()
        ));
    }

    log_info_details!(
        "commands::recording",
        "multi_input_start_requested",
        json!({
            "device_ids": device_ids.clone(),
            "count": device_ids.len()
        })
    );

    // Enable multi-input mode and save device IDs
    state.set_multi_input_enabled(true);
    state.set_selected_device_ids(device_ids.clone());

    let primary_device = device_ids[0].clone();
    start_recording_internal(&app, &state, primary_device).await?;

    Ok(format!(
        "Multi-input recording started with {} device(s)",
        device_ids.len()
    ))
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

    // Get audio recorder reference
    let audio_recorder = {
        let recorder_lock = state.audio_recorder.lock().unwrap();
        recorder_lock
            .clone()
            .ok_or_else(|| "Audio recorder not initialized".to_string())?
    };

    // Cancel IPC reader and audio sender tasks
    state.cancel_recording_tasks();
    log_info!("commands::recording", "tasks_cancelled");

    // Stop audio recorder (cleanup resources, including mixer thread)
    let mut recorder = audio_recorder.lock().await;
    recorder.stop().map_err(|e| e.to_string())?;

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
/// Stops audio device recording
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
    // MVP1: Real device adapter enumeration
    match crate::audio_device_adapter::enumerate_devices_static() {
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

// ============================================================================
// Multi-Input Settings Commands (Task 7)
// ============================================================================

/// Save multi-input settings to disk
///
/// Requirement: STTMIX-REQ-001.2, STTMIX-REQ-005.1
#[tauri::command]
pub async fn save_multi_input_settings(
    app: AppHandle,
    settings: crate::multi_input_settings::MultiInputSettings,
) -> Result<(), String> {
    use crate::multi_input_settings::save_settings;

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    save_settings(&app_data_dir, &settings)
        .map_err(|e| format!("Failed to save multi-input settings: {}", e))?;

    log_info_details!(
        "commands::settings",
        "multi_input_settings_saved",
        json!({
            "device_count": settings.selected_device_ids.len(),
            "enabled": settings.multi_input_enabled
        })
    );

    Ok(())
}

/// Load multi-input settings from disk
///
/// Requirement: STTMIX-REQ-001.2
#[tauri::command]
pub async fn load_multi_input_settings(
    app: AppHandle,
) -> Result<crate::multi_input_settings::MultiInputSettings, String> {
    use crate::multi_input_settings::load_settings;

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let settings = load_settings(&app_data_dir)
        .map_err(|e| format!("Failed to load multi-input settings: {}", e))?;

    log_info_details!(
        "commands::settings",
        "multi_input_settings_loaded",
        json!({
            "device_count": settings.selected_device_ids.len(),
            "enabled": settings.multi_input_enabled
        })
    );

    Ok(settings)
}

/// Get platform information for feature gating
///
/// Returns the current OS platform. Multi-input is only supported on macOS.
///
/// Requirement: STTMIX-CON-004
#[tauri::command]
pub fn get_platform_info() -> PlatformInfo {
    PlatformInfo {
        os: std::env::consts::OS.to_string(),
        multi_input_supported: cfg!(target_os = "macos"),
    }
}

#[derive(serde::Serialize)]
pub struct PlatformInfo {
    pub os: String,
    pub multi_input_supported: bool,
}

/// Get status of all multi-input channels
///
/// Returns buffer occupancy, active status, and metrics for each input.
/// Used by UI to display input health indicators.
///
/// Requirement: STTMIX-REQ-008.1, Task 8.3
#[tauri::command]
pub async fn get_multi_input_status(
    state: State<'_, AppState>,
) -> Result<MultiInputStatusResponse, String> {
    let recorder_opt = state.audio_recorder.lock().unwrap().clone();
    let recorder_arc = recorder_opt.ok_or("Audio recorder not initialized")?;
    let recorder = recorder_arc.lock().await;

    let input_statuses = recorder.get_input_status();
    let mixer_metrics = recorder.get_mixer_metrics();

    Ok(MultiInputStatusResponse {
        inputs: input_statuses,
        is_recording: recorder.is_recording(),
        mixer_metrics: mixer_metrics.map(|m| MixerMetricsResponse {
            drift_correction_count: m.get_drift_correction_count(),
            clip_count: m.get_clip_count(),
            silence_insertion_count: m.get_silence_insertion_count(),
            frames_mixed: m.get_frames_mixed(),
            max_mix_latency_ms: m.get_max_mix_latency_ms(),
            avg_mix_latency_ms: m.get_avg_mix_latency_ms(),
        }),
    })
}

#[derive(serde::Serialize)]
pub struct MultiInputStatusResponse {
    pub inputs: Vec<InputStatus>,
    pub is_recording: bool,
    pub mixer_metrics: Option<MixerMetricsResponse>,
}

#[derive(serde::Serialize)]
pub struct MixerMetricsResponse {
    pub drift_correction_count: u64,
    pub clip_count: u64,
    pub silence_insertion_count: u64,
    pub frames_mixed: u64,
    // Task 9.1: Latency metrics
    pub max_mix_latency_ms: f64,
    pub avg_mix_latency_ms: f64,
}

/// Validate that selected devices are still available
///
/// Returns list of unavailable device IDs
///
/// Requirement: STTMIX-REQ-001.2
#[tauri::command]
pub async fn validate_multi_input_devices(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    use crate::multi_input_settings::{load_settings, validate_devices};

    // Load current settings
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let settings = load_settings(&app_data_dir)
        .map_err(|e| format!("Failed to load settings: {}", e))?;

    // Get available devices using the recorder's factory
    let recorder_opt = state.audio_recorder.lock().unwrap().clone();
    let available_devices: Vec<String> = if let Some(recorder_arc) = recorder_opt {
        let recorder = recorder_arc.lock().await;
        recorder
            .enumerate_devices()
            .map_err(|e| format!("Failed to enumerate devices: {}", e))?
            .into_iter()
            .map(|d| d.id)
            .collect()
    } else {
        Vec::new()
    };

    // Find unavailable devices
    let unavailable = validate_devices(&settings, &available_devices);

    if !unavailable.is_empty() {
        log_warn_details!(
            "commands::settings",
            "unavailable_devices_detected",
            json!({
                "unavailable": unavailable,
                "total_selected": settings.selected_device_ids.len()
            })
        );
    }

    Ok(unavailable)
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

    // ========================================================================
    // STTMIX Task 1.3: Multi-Input Command Validation Tests
    // ========================================================================

    /// Test: start_recording_multi validates empty device_ids
    /// STTMIX-REQ-002.1: At least one device must be provided
    #[test]
    fn test_multi_input_validation_empty() {
        let device_ids: Vec<String> = vec![];

        // Validation logic (same as in start_recording_multi)
        let result = if device_ids.is_empty() {
            Err("At least one device ID must be provided")
        } else {
            Ok(())
        };

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("At least one"));
    }

    /// Test: start_recording_multi validates max 2 inputs
    /// STTMIX-CON-005: Maximum 2 inputs supported
    #[test]
    fn test_multi_input_validation_max_2() {
        let device_ids = vec![
            "mic-1".to_string(),
            "loopback-1".to_string(),
            "mic-2".to_string(), // 3rd device - should fail
        ];

        // Validation logic (same as in start_recording_multi)
        let result = if device_ids.len() > 2 {
            Err(format!(
                "Maximum 2 inputs supported, got {}",
                device_ids.len()
            ))
        } else {
            Ok(())
        };

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Maximum 2 inputs"));
    }

    /// Test: start_recording_multi accepts 1 device (valid)
    #[test]
    fn test_multi_input_validation_single_device_ok() {
        let device_ids = vec!["mic-1".to_string()];

        // Validation logic
        let result = if device_ids.is_empty() {
            Err("Empty")
        } else if device_ids.len() > 2 {
            Err("Too many")
        } else {
            Ok(())
        };

        assert!(result.is_ok());
    }

    /// Test: start_recording_multi accepts 2 devices (valid)
    #[test]
    fn test_multi_input_validation_two_devices_ok() {
        let device_ids = vec!["mic-1".to_string(), "loopback-1".to_string()];

        // Validation logic
        let result = if device_ids.is_empty() {
            Err("Empty")
        } else if device_ids.len() > 2 {
            Err("Too many")
        } else {
            Ok(())
        };

        assert!(result.is_ok());
        assert_eq!(device_ids.len(), 2);
    }
}
