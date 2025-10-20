// Legacy IPC Message Type (Test Support Only)
// Moved from src/python_sidecar.rs (Task 14.1 - Post-MVP1 Cleanup)
//
// This module preserves ADR-003 backward compatibility verification
// while keeping production code free of deprecated types.

use meeting_minutes_automator_lib::ipc_protocol::{
    IpcMessage as ProtocolMessage, TranscriptionResult, PROTOCOL_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, to_value};

/// Legacy IPC message types (MVP0 - Walking Skeleton)
/// DEPRECATED: Use crate::ipc_protocol::IpcMessage instead
/// Kept for backward compatibility testing (ADR-003)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum LegacyIpcMessage {
    /// Request to start processing audio
    StartProcessing { audio_data: Vec<u8> },

    /// Request to stop processing
    StopProcessing,

    /// Response with transcription result
    TranscriptionResult { text: String, timestamp: u64 },

    /// Ready signal from Python
    Ready,

    /// Error from Python
    Error { message: String },
}

impl LegacyIpcMessage {
    /// Convert legacy message to new protocol message
    /// Used for backward compatibility testing during migration
    pub fn to_protocol_message(&self) -> ProtocolMessage {
        match self {
            LegacyIpcMessage::TranscriptionResult { text, .. } => {
                let transcription = TranscriptionResult {
                    text: text.clone(),
                    is_final: true,
                    confidence: None,
                    language: None,
                    processing_time_ms: None,
                    model_size: None,
                };
                ProtocolMessage::Response {
                    id: "legacy".to_string(),
                    version: PROTOCOL_VERSION.to_string(),
                    result: to_value(&transcription).unwrap(),
                }
            }
            LegacyIpcMessage::Error { message } => ProtocolMessage::Error {
                id: "legacy-error".to_string(),
                version: PROTOCOL_VERSION.to_string(),
                error_code: "LEGACY_ERROR".to_string(),
                error_message: message.clone(),
                recoverable: true,
            },
            LegacyIpcMessage::Ready => {
                // Ready is not part of new protocol, convert to Response
                let transcription = TranscriptionResult {
                    text: "ready".to_string(),
                    is_final: true,
                    confidence: None,
                    language: None,
                    processing_time_ms: None,
                    model_size: None,
                };
                ProtocolMessage::Response {
                    id: "ready".to_string(),
                    version: PROTOCOL_VERSION.to_string(),
                    result: to_value(&transcription).unwrap(),
                }
            }
            LegacyIpcMessage::StartProcessing { audio_data } => {
                // StartProcessing is equivalent to process_audio in new protocol
                ProtocolMessage::Request {
                    id: "legacy-request".to_string(),
                    version: PROTOCOL_VERSION.to_string(),
                    method: "process_audio".to_string(),
                    params: json!({ "audio_data": audio_data }),
                }
            }
            LegacyIpcMessage::StopProcessing => {
                // StopProcessing has no direct equivalent in new protocol
                // In new protocol, recording stop is handled by Rust side only
                // Return a no-op request for compatibility
                ProtocolMessage::Request {
                    id: "legacy-request".to_string(),
                    version: PROTOCOL_VERSION.to_string(),
                    method: "stop_processing".to_string(),
                    params: json!({}),
                }
            }
        }
    }
}
