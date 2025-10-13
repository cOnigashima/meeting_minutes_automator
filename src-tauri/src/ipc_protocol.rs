//! IPC Protocol Module (STT-REQ-007 series)
//!
//! Rust-Python IPC通信の後方互換性を維持しながら、拡張フィールドを追加。
//!
//! ## 設計決定
//! - **後方互換性**: `#[serde(default = "default_version")]`で旧形式メッセージ受信
//! - **前方互換性**: `deny_unknown_fields`未使用で未知フィールド無視
//! - **関連ADR**: ADR-003（リソースベースモデル選択とIPC Version Strategy）
//!
//! ## 関連要件
//! - STT-REQ-007.1: 後方互換性維持（LegacyIpcMessage Fallback）
//! - STT-REQ-007.2: 拡張フィールド（confidence, language, processing_time_ms, model_size）
//! - STT-REQ-007.4: versionフィールド必須化（PROTOCOL_VERSION = "1.0"）
//! - STT-REQ-007.5: エラーフォーマット統一（errorCode, errorMessage, recoverable）
//!
//! ## 使用例
//! ```rust
//! use meeting_minutes_automator::ipc_protocol::{IpcMessage, TranscriptionResult, PROTOCOL_VERSION};
//!
//! // 新形式Requestの送信
//! let request = IpcMessage::Request {
//!     id: "req-123".to_string(),
//!     version: PROTOCOL_VERSION.to_string(),
//!     method: "process_audio".to_string(),
//!     params: serde_json::json!({"audio_data": [0u8, 1, 2]}),
//! };
//!
//! // 新形式Responseの受信
//! let response_json = r#"{"type":"response","id":"resp-456","version":"1.0","result":{"text":"こんにちは","is_final":true}}"#;
//! let response: IpcMessage = serde_json::from_str(response_json)?;
//! ```
//!
//! Task 7.1: IPC Message Extension and Versioning
//! Task 7.1.5: Integration with existing IPC communication (commands.rs, python_sidecar.rs)

use serde::{Deserialize, Serialize};

/// Protocol version constant (STT-REQ-007.4)
pub const PROTOCOL_VERSION: &str = "1.0";

fn default_version() -> String {
    PROTOCOL_VERSION.to_string()
}

/// Check version compatibility according to STT-REQ-007.6
///
/// Semantic versioning rules:
/// - Major version mismatch (1.x → 2.x): MajorMismatch (error + reject)
/// - Minor version mismatch (1.0 → 1.1): MinorMismatch (warning + backward compat)
/// - Patch version mismatch (1.0.1 → 1.0.2): Compatible (info log only)
///
/// # Arguments
/// * `received` - Version string from IPC message
/// * `expected` - Expected protocol version (PROTOCOL_VERSION)
pub fn check_version_compatibility(received: &str, expected: &str) -> VersionCompatibility {
    // Parse received version
    let received_parts: Vec<&str> = received.split('.').collect();
    let expected_parts: Vec<&str> = expected.split('.').collect();

    // Validate version format (at least major.minor)
    if received_parts.len() < 2 || expected_parts.len() < 2 {
        return VersionCompatibility::Malformed {
            received: received.to_string(),
        };
    }

    // Parse major and minor versions
    let received_major = match received_parts[0].parse::<u32>() {
        Ok(v) => v,
        Err(_) => {
            return VersionCompatibility::Malformed {
                received: received.to_string(),
            }
        }
    };

    let received_minor = match received_parts[1].parse::<u32>() {
        Ok(v) => v,
        Err(_) => {
            return VersionCompatibility::Malformed {
                received: received.to_string(),
            }
        }
    };

    let expected_major = expected_parts[0].parse::<u32>().unwrap();
    let expected_minor = expected_parts[1].parse::<u32>().unwrap();

    // Check major version (STT-REQ-007.6: Major mismatch → reject)
    if received_major != expected_major {
        return VersionCompatibility::MajorMismatch {
            received: received.to_string(),
            expected: expected.to_string(),
        };
    }

    // Check minor version (STT-REQ-007.6: Minor mismatch → warning + backward compat)
    if received_minor != expected_minor {
        return VersionCompatibility::MinorMismatch {
            received: received.to_string(),
            expected: expected.to_string(),
        };
    }

    // Patch version difference is ignored (STT-REQ-007.6: Patch → info log only)
    VersionCompatibility::Compatible
}

/// Transcription result with extended fields
/// Related requirement: STT-REQ-007.2
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranscriptionResult {
    /// Transcribed text
    pub text: String,

    /// Whether this is a final result (true) or partial (false)
    pub is_final: bool,

    /// Confidence score (0.0-1.0), optional for backward compatibility
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,

    /// Detected language code (e.g., "ja", "en")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Processing time in milliseconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processing_time_ms: Option<u64>,

    /// Model size used (e.g., "small", "medium", "large-v3")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_size: Option<String>,
}

/// IPC message types for communication with Python sidecar
/// Related requirement: STT-REQ-007.1, STT-REQ-007.5
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum IpcMessage {
    /// Request message
    #[serde(rename = "request")]
    Request {
        id: String,
        #[serde(default = "default_version")]
        version: String,
        method: String,
        params: serde_json::Value,
    },

    /// Response message with generic result
    /// The result field can be either:
    /// - TranscriptionResult (for process_audio): {text, is_final, ...}
    /// - Generic data (for approve_upgrade): {success, old_model, new_model}
    /// Caller must check result structure and parse accordingly
    #[serde(rename = "response")]
    Response {
        id: String,
        #[serde(default = "default_version")]
        version: String,
        result: serde_json::Value,
    },

    /// Error message
    /// Related requirement: STT-REQ-007.5
    #[serde(rename = "error")]
    Error {
        id: String,
        #[serde(default = "default_version")]
        version: String,
        #[serde(rename = "errorCode")]
        error_code: String,
        #[serde(rename = "errorMessage")]
        error_message: String,
        recoverable: bool,
    },

    /// Event notification message (non-request-response)
    /// Used for asynchronous notifications from Python to Rust
    /// Examples: model_change, upgrade_proposal, recording_paused
    #[serde(rename = "event")]
    Event {
        #[serde(default = "default_version")]
        version: String,
        #[serde(rename = "eventType")]
        event_type: String,
        data: serde_json::Value,
    },
}

/// Version compatibility check result
/// Requirement: STT-REQ-007.6
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionCompatibility {
    /// Versions are fully compatible
    Compatible,
    /// Minor version mismatch (warning log, backward compat mode)
    MinorMismatch { received: String, expected: String },
    /// Major version mismatch (error, reject communication)
    MajorMismatch { received: String, expected: String },
    /// Malformed version string (error, reject communication)
    Malformed { received: String },
}

impl IpcMessage {
    /// Get the version from any message type
    pub fn version(&self) -> &str {
        match self {
            IpcMessage::Request { version, .. } => version,
            IpcMessage::Response { version, .. } => version,
            IpcMessage::Error { version, .. } => version,
            IpcMessage::Event { version, .. } => version,
        }
    }

    /// Get the message ID (returns "N/A" for Event messages which don't have IDs)
    pub fn id(&self) -> &str {
        match self {
            IpcMessage::Request { id, .. } => id,
            IpcMessage::Response { id, .. } => id,
            IpcMessage::Error { id, .. } => id,
            IpcMessage::Event { .. } => "N/A", // Events don't have request IDs
        }
    }

    /// Check version compatibility according to STT-REQ-007.6
    ///
    /// - Major version mismatch (1.x → 2.x): Error + Reject
    /// - Minor version mismatch (1.0 → 1.1): Warning + Backward compat
    /// - Patch version mismatch (1.0.1 → 1.0.2): Info log only
    pub fn check_version_compatibility(&self) -> VersionCompatibility {
        check_version_compatibility(self.version(), PROTOCOL_VERSION)
    }

    /// Try to parse result as TranscriptionResult
    pub fn as_transcription_result(&self) -> Option<TranscriptionResult> {
        match self {
            IpcMessage::Response { result, .. } => {
                serde_json::from_value(result.clone()).ok()
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================================
    // Task 7.1: RED - 失敗するテスト（コンパイルエラー含む）
    // Related requirement: STT-REQ-007.2, STT-REQ-007.4
    // ================================================================================

    #[test]
    fn test_transcription_result_with_all_fields() {
        // Arrange
        let result = TranscriptionResult {
            text: "こんにちは".to_string(),
            is_final: true,
            confidence: Some(0.95),
            language: Some("ja".to_string()),
            processing_time_ms: Some(450),
            model_size: Some("small".to_string()),
        };

        // Act: JSON serialization
        let json = serde_json::to_string(&result).unwrap();

        // Assert: All fields are present
        assert!(json.contains("\"text\":\"こんにちは\""));
        assert!(json.contains("\"is_final\":true"));
        assert!(json.contains("\"confidence\":0.95"));
        assert!(json.contains("\"language\":\"ja\""));
        assert!(json.contains("\"processing_time_ms\":450"));
        assert!(json.contains("\"model_size\":\"small\""));
    }

    #[test]
    fn test_transcription_result_backward_compatibility() {
        // Arrange: Old format JSON (without new fields)
        let json = r#"{"text":"test","is_final":true}"#;

        // Act: Deserialize
        let result: TranscriptionResult = serde_json::from_str(json).unwrap();

        // Assert: New fields are None (backward compatible)
        assert_eq!(result.text, "test");
        assert_eq!(result.is_final, true);
        assert_eq!(result.confidence, None);
        assert_eq!(result.language, None);
        assert_eq!(result.processing_time_ms, None);
        assert_eq!(result.model_size, None);
    }

    #[test]
    fn test_ipc_message_response_with_version() {
        // Arrange
        let transcription = TranscriptionResult {
            text: "Hello world".to_string(),
            is_final: true,
            confidence: Some(0.98),
            language: Some("en".to_string()),
            processing_time_ms: Some(250),
            model_size: Some("medium".to_string()),
        };
        let msg = IpcMessage::Response {
            id: "test-123".to_string(),
            version: PROTOCOL_VERSION.to_string(),
            result: serde_json::to_value(&transcription).unwrap(),
        };

        // Act: JSON serialization
        let json = serde_json::to_string(&msg).unwrap();

        // Assert: Version field is present
        assert!(json.contains("\"type\":\"response\""));
        assert!(json.contains("\"version\":\"1.0\""));
        assert!(json.contains("\"id\":\"test-123\""));
    }

    #[test]
    fn test_ipc_message_error_format() {
        // Arrange: STT-REQ-007.5
        let msg = IpcMessage::Error {
            id: "error-456".to_string(),
            version: PROTOCOL_VERSION.to_string(),
            error_code: "STT_INFERENCE_ERROR".to_string(),
            error_message: "Whisper model inference failed: timeout".to_string(),
            recoverable: false,
        };

        // Act: JSON serialization
        let json = serde_json::to_string(&msg).unwrap();

        // Assert: Error format matches STT-REQ-007.5
        assert!(json.contains("\"type\":\"error\""));
        assert!(json.contains("\"errorCode\":\"STT_INFERENCE_ERROR\""));
        assert!(json.contains("\"errorMessage\":\"Whisper model inference failed: timeout\""));
        assert!(json.contains("\"recoverable\":false"));
    }

    #[test]
    fn test_ipc_message_version_accessor() {
        // Arrange
        let transcription = TranscriptionResult {
            text: "test".to_string(),
            is_final: true,
            confidence: None,
            language: None,
            processing_time_ms: None,
            model_size: None,
        };
        let msg = IpcMessage::Response {
            id: "test".to_string(),
            version: "1.0".to_string(),
            result: serde_json::to_value(&transcription).unwrap(),
        };

        // Act & Assert
        assert_eq!(msg.version(), "1.0");
        assert_eq!(msg.id(), "test");
    }

    #[test]
    fn test_transcription_result_skip_none_fields() {
        // Arrange: Result with only required fields
        let result = TranscriptionResult {
            text: "minimal".to_string(),
            is_final: false,
            confidence: None,
            language: None,
            processing_time_ms: None,
            model_size: None,
        };

        // Act: JSON serialization
        let json = serde_json::to_string(&result).unwrap();

        // Assert: None fields are skipped
        assert!(json.contains("\"text\":\"minimal\""));
        assert!(json.contains("\"is_final\":false"));
        assert!(!json.contains("\"confidence\""));
        assert!(!json.contains("\"language\""));
        assert!(!json.contains("\"processing_time_ms\""));
        assert!(!json.contains("\"model_size\""));
    }

    #[test]
    fn test_ipc_message_roundtrip() {
        // Arrange
        let transcription = TranscriptionResult {
            text: "roundtrip test".to_string(),
            is_final: true,
            confidence: Some(0.92),
            language: Some("en".to_string()),
            processing_time_ms: Some(300),
            model_size: Some("small".to_string()),
        };
        let original = IpcMessage::Response {
            id: "roundtrip-789".to_string(),
            version: "1.0".to_string(),
            result: serde_json::to_value(&transcription).unwrap(),
        };

        // Act: Serialize then deserialize
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: IpcMessage = serde_json::from_str(&json).unwrap();

        // Assert: JSON roundtrip preserves structure
        // Note: Direct equality fails due to f32 precision, so verify fields instead
        match (original, deserialized) {
            (
                IpcMessage::Response {
                    id: id1,
                    version: v1,
                    result: r1,
                },
                IpcMessage::Response {
                    id: id2,
                    version: v2,
                    result: r2,
                },
            ) => {
                assert_eq!(id1, id2);
                assert_eq!(v1, v2);
                assert_eq!(r1["text"], r2["text"]);
                assert_eq!(r1["is_final"], r2["is_final"]);
                assert_eq!(r1["language"], r2["language"]);
                assert_eq!(r1["processing_time_ms"], r2["processing_time_ms"]);
                assert_eq!(r1["model_size"], r2["model_size"]);
                // Confidence may have f32 precision differences, check approximate equality
                if let (Some(c1), Some(c2)) =
                    (r1["confidence"].as_f64(), r2["confidence"].as_f64())
                {
                    assert!((c1 - c2).abs() < 0.0001, "Confidence mismatch: {} vs {}", c1, c2);
                }
            }
            _ => panic!("Expected Response variants"),
        }
    }

    #[test]
    fn test_forward_compatibility_ignore_unknown_fields() {
        // Arrange: JSON with unknown field "future_field"
        let json = r#"{"type":"response","id":"test","version":"1.0","result":{"text":"test","is_final":true,"future_field":"ignored"}}"#;

        // Act: Deserialize (should not fail)
        let msg: IpcMessage = serde_json::from_str(json).unwrap();

        // Assert: Unknown field is ignored
        if let IpcMessage::Response { result, .. } = msg {
            let transcription: TranscriptionResult = serde_json::from_value(result).unwrap();
            assert_eq!(transcription.text, "test");
            assert_eq!(transcription.is_final, true);
        } else {
            panic!("Expected Response variant");
        }
    }

    #[test]
    fn test_version_constant() {
        // Assert: PROTOCOL_VERSION is "1.0" (STT-REQ-007.4)
        assert_eq!(PROTOCOL_VERSION, "1.0");
    }

    #[test]
    fn test_confidence_range() {
        // Arrange: Confidence scores at boundaries
        let results = vec![
            TranscriptionResult {
                text: "low confidence".to_string(),
                is_final: true,
                confidence: Some(0.0),
                language: None,
                processing_time_ms: None,
                model_size: None,
            },
            TranscriptionResult {
                text: "high confidence".to_string(),
                is_final: true,
                confidence: Some(1.0),
                language: None,
                processing_time_ms: None,
                model_size: None,
            },
        ];

        // Act & Assert: Serialization succeeds
        for result in results {
            let json = serde_json::to_string(&result);
            assert!(json.is_ok());
        }
    }

    #[test]
    fn test_version_field_omitted_defaults_to_1_0() {
        // Arrange: Old format JSON without version field (backward compatibility with MVP0)
        // Related requirement: ADR-003 (IPC Version Strategy)
        let response_json = r#"{"type":"response","id":"test-old","result":{"text":"test","is_final":true}}"#;
        let error_json = r#"{"type":"error","id":"error-old","errorCode":"TEST_ERROR","errorMessage":"test error","recoverable":true}"#;

        // Act: Deserialize messages without version field
        let response: IpcMessage = serde_json::from_str(response_json).unwrap();
        let error: IpcMessage = serde_json::from_str(error_json).unwrap();

        // Assert: Version defaults to "1.0"
        assert_eq!(response.version(), "1.0");
        assert_eq!(error.version(), "1.0");
    }

    #[test]
    fn test_event_message_format() {
        // Arrange
        let event = IpcMessage::Event {
            version: "1.0".to_string(),
            event_type: "model_change".to_string(),
            data: serde_json::json!({
                "old_model": "small",
                "new_model": "tiny",
                "reason": "cpu_high"
            }),
        };

        // Act: JSON serialization
        let json = serde_json::to_string(&event).unwrap();

        // Assert: Event format is correct
        assert!(json.contains("\"type\":\"event\""));
        assert!(json.contains("\"version\":\"1.0\""));
        assert!(json.contains("\"eventType\":\"model_change\""));
        assert!(json.contains("\"data\""));
        assert!(json.contains("\"old_model\":\"small\""));
    }

    #[test]
    fn test_event_message_roundtrip() {
        // Arrange
        let original = IpcMessage::Event {
            version: "1.0".to_string(),
            event_type: "upgrade_proposal".to_string(),
            data: serde_json::json!({
                "current_model": "tiny",
                "proposed_model": "small",
                "message": "Upgrade available"
            }),
        };

        // Act: Serialize then deserialize
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: IpcMessage = serde_json::from_str(&json).unwrap();

        // Assert: Perfect roundtrip
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_event_message_version_accessor() {
        // Arrange
        let event = IpcMessage::Event {
            version: "1.0".to_string(),
            event_type: "test_event".to_string(),
            data: serde_json::json!({}),
        };

        // Act & Assert
        assert_eq!(event.version(), "1.0");
        assert_eq!(event.id(), "N/A"); // Events don't have IDs
    }

    #[test]
    fn test_event_message_version_default() {
        // Arrange: Event without version field (old format)
        let json = r#"{"type":"event","eventType":"test","data":{}}"#;

        // Act: Deserialize
        let msg: IpcMessage = serde_json::from_str(json).unwrap();

        // Assert: Version defaults to "1.0"
        assert_eq!(msg.version(), "1.0");
    }
}
