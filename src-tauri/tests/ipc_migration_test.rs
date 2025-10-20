// Integration Tests: IPC Protocol Migration
// Task 7.1.5: Verify new IPC protocol integration and backward compatibility
// Task 7.2: Version compatibility checking (STT-REQ-007.6)
// Requirements: STT-REQ-007.1, STT-REQ-007.2, STT-REQ-007.4, STT-REQ-007.6, ADR-003

#[path = "support/mod.rs"]
mod support;

use support::LegacyIpcMessage;
use meeting_minutes_automator_lib::ipc_protocol::{
    check_version_compatibility, IpcMessage, TranscriptionResult, VersionCompatibility,
    PROTOCOL_VERSION,
};
use serde_json::json;

/// Test: New IPC format roundtrip (Rust ↔ JSON ↔ Rust)
/// Requirement: STT-REQ-007.1, STT-REQ-007.4
#[test]
fn test_new_ipc_format_roundtrip() {
    // Arrange
    let transcription = TranscriptionResult {
        text: "こんにちは、世界".to_string(),
        is_final: true,
        confidence: Some(0.95),
        language: Some("ja".to_string()),
        processing_time_ms: Some(450),
        model_size: Some("small".to_string()),
    };
    let original = IpcMessage::Response {
        id: "test-123".to_string(),
        version: PROTOCOL_VERSION.to_string(),
        result: serde_json::to_value(&transcription).unwrap(),
    };

    // Act: Serialize → Deserialize
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: IpcMessage = serde_json::from_str(&json).unwrap();

    // Assert: Perfect roundtrip
    assert_eq!(original, deserialized);
}

/// Test: Legacy format cannot be parsed as new format (expected failure)
/// Requirement: ADR-003 (backward compatibility strategy)
#[test]
fn test_legacy_format_not_parsed_as_new_format() {
    // Arrange: Legacy format (top-level text, no version)
    let legacy_json = r#"{"text":"test","timestamp":123456}"#;

    // Act: Try to parse as new format
    let result = serde_json::from_str::<IpcMessage>(legacy_json);

    // Assert: Should fail (legacy format is incompatible)
    assert!(
        result.is_err(),
        "Legacy format should not parse as new IpcMessage"
    );
}

/// Test: New format request serialization (Rust → Python)
/// Requirement: STT-REQ-007.1, STT-REQ-007.4
#[test]
fn test_new_format_request_serialization() {
    // Arrange
    let request = IpcMessage::Request {
        id: "req-456".to_string(),
        version: "1.0".to_string(),
        method: "process_audio".to_string(),
        params: json!({ "audio_data": [0u8, 1, 2, 3, 4] }),
    };

    // Act: Serialize
    let json = serde_json::to_string(&request).unwrap();

    // Assert: Contains required fields
    assert!(json.contains("\"type\":\"request\""));
    assert!(json.contains("\"id\":\"req-456\""));
    assert!(json.contains("\"version\":\"1.0\""));
    assert!(json.contains("\"method\":\"process_audio\""));
    assert!(json.contains("\"params\""));
}

/// Test: New format error handling
/// Requirement: STT-REQ-007.5
#[test]
fn test_new_format_error_response() {
    // Arrange
    let error = IpcMessage::Error {
        id: "err-789".to_string(),
        version: "1.0".to_string(),
        error_code: "STT_INFERENCE_ERROR".to_string(),
        error_message: "Whisper model inference failed".to_string(),
        recoverable: false,
    };

    // Act: Serialize → Deserialize
    let json = serde_json::to_string(&error).unwrap();
    let deserialized: IpcMessage = serde_json::from_str(&json).unwrap();

    // Assert
    assert_eq!(error, deserialized);
    assert!(json.contains("\"errorCode\":\"STT_INFERENCE_ERROR\""));
    assert!(json.contains("\"recoverable\":false"));
}

/// Test: Legacy to new format conversion
/// Requirement: ADR-003 (migration strategy)
#[test]
#[allow(deprecated)]
fn test_legacy_to_new_format_conversion() {
    // Arrange: Legacy transcription result
    let legacy = LegacyIpcMessage::TranscriptionResult {
        text: "legacy text".to_string(),
        timestamp: 123456,
    };

    // Act: Convert to new format
    let new_format = legacy.to_protocol_message();

    // Assert: Proper conversion
    match new_format {
        IpcMessage::Response {
            id,
            version,
            result,
        } => {
            assert_eq!(id, "legacy");
            assert_eq!(version, "1.0");

            // Parse result as TranscriptionResult
            let transcription: TranscriptionResult = serde_json::from_value(result).unwrap();
            assert_eq!(transcription.text, "legacy text");
            assert_eq!(transcription.is_final, true);
            assert_eq!(transcription.confidence, None); // Legacy format doesn't have confidence
            assert_eq!(transcription.language, None);
            assert_eq!(transcription.processing_time_ms, None);
            assert_eq!(transcription.model_size, None);
        }
        _ => panic!("Expected Response variant"),
    }
}

/// Test: Backward compatibility - version field omitted defaults to "1.0"
/// Requirement: ADR-003
#[test]
fn test_version_field_omitted_backward_compat() {
    // Arrange: JSON without version field (old format)
    let json_no_version = r#"{
        "type": "response",
        "id": "old-msg",
        "result": {
            "text": "test",
            "is_final": true
        }
    }"#;

    // Act: Deserialize
    let msg: IpcMessage = serde_json::from_str(json_no_version).unwrap();

    // Assert: Version defaults to "1.0"
    assert_eq!(msg.version(), "1.0");
}

/// Test: Forward compatibility - ignore unknown fields
/// Requirement: STT-REQ-007.1
#[test]
fn test_forward_compatibility_unknown_fields() {
    // Arrange: JSON with future field "future_field"
    let json_with_future = r#"{
        "type": "response",
        "id": "future-msg",
        "version": "1.0",
        "result": {
            "text": "test",
            "is_final": true,
            "future_field": "ignored"
        }
    }"#;

    // Act: Deserialize (should not fail)
    let msg: IpcMessage = serde_json::from_str(json_with_future).unwrap();

    // Assert: Unknown field is ignored
    match msg {
        IpcMessage::Response { result, .. } => {
            let transcription: TranscriptionResult = serde_json::from_value(result).unwrap();
            assert_eq!(transcription.text, "test");
            assert_eq!(transcription.is_final, true);
        }
        _ => panic!("Expected Response variant"),
    }
}

/// Test: Extended fields serialization (Task 7.1 feature)
/// Requirement: STT-REQ-007.2
#[test]
fn test_extended_fields_serialization() {
    // Arrange: Response with all extended fields
    let transcription = TranscriptionResult {
        text: "Extended test".to_string(),
        is_final: true,
        confidence: Some(0.98),
        language: Some("en".to_string()),
        processing_time_ms: Some(250),
        model_size: Some("medium".to_string()),
    };
    let response = IpcMessage::Response {
        id: "extended-test".to_string(),
        version: "1.0".to_string(),
        result: serde_json::to_value(&transcription).unwrap(),
    };

    // Act: Serialize
    let json = serde_json::to_string(&response).unwrap();

    // Assert: All extended fields are present
    assert!(json.contains("\"confidence\":0.98") || json.contains("\"confidence\":0.9")); // Float precision tolerance
    assert!(json.contains("\"language\":\"en\""));
    assert!(json.contains("\"processing_time_ms\":250"));
    assert!(json.contains("\"model_size\":\"medium\""));
}

/// Test: Extended fields omitted when None (minimize JSON size)
/// Requirement: STT-REQ-007.2
#[test]
fn test_extended_fields_omitted_when_none() {
    // Arrange: Response with minimal fields
    let transcription = TranscriptionResult {
        text: "Minimal".to_string(),
        is_final: false,
        confidence: None,
        language: None,
        processing_time_ms: None,
        model_size: None,
    };
    let response = IpcMessage::Response {
        id: "minimal-test".to_string(),
        version: "1.0".to_string(),
        result: serde_json::to_value(&transcription).unwrap(),
    };

    // Act: Serialize
    let json = serde_json::to_string(&response).unwrap();

    // Assert: None fields are not serialized
    assert!(!json.contains("\"confidence\""));
    assert!(!json.contains("\"language\""));
    assert!(!json.contains("\"processing_time_ms\""));
    assert!(!json.contains("\"model_size\""));
}

/// Test: Python-like response format verification
/// Requirement: STT-REQ-007.2 (Python↔Rust communication)
#[test]
fn test_python_response_format_compatibility() {
    // Arrange: Simulate Python's new format response
    let python_json = r#"{
        "id": "audio-123",
        "type": "response",
        "version": "1.0",
        "result": {
            "text": "こんにちは",
            "is_final": true,
            "confidence": 0.95,
            "language": "ja",
            "processing_time_ms": 450,
            "model_size": "small"
        }
    }"#;

    // Act: Parse as Rust IpcMessage
    let msg: IpcMessage = serde_json::from_str(python_json).unwrap();

    // Assert: Correctly parsed
    match msg {
        IpcMessage::Response {
            id,
            version,
            result,
        } => {
            assert_eq!(id, "audio-123");
            assert_eq!(version, "1.0");

            // Parse result as TranscriptionResult
            let transcription: TranscriptionResult = serde_json::from_value(result).unwrap();
            assert_eq!(transcription.text, "こんにちは");
            assert_eq!(transcription.is_final, true);
            assert_eq!(transcription.confidence, Some(0.95));
            assert_eq!(transcription.language, Some("ja".to_string()));
            assert_eq!(transcription.processing_time_ms, Some(450));
            assert_eq!(transcription.model_size, Some("small".to_string()));
        }
        _ => panic!("Expected Response variant"),
    }
}

/// Test: Rust request to Python format verification
/// Requirement: STT-REQ-007.1 (Rust→Python communication)
#[test]
fn test_rust_request_format_for_python() {
    // Arrange: Create Rust request
    use meeting_minutes_automator_lib::ipc_protocol::PROTOCOL_VERSION;

    let request = IpcMessage::Request {
        id: "audio-456".to_string(),
        version: PROTOCOL_VERSION.to_string(),
        method: "process_audio".to_string(),
        params: serde_json::json!({"audio_data": [0u8, 1, 2, 3]}),
    };

    // Act: Serialize to JSON (what Python will receive)
    let json = serde_json::to_string(&request).unwrap();

    // Assert: Python can parse this
    assert!(json.contains("\"type\":\"request\""));
    assert!(json.contains("\"method\":\"process_audio\""));
    assert!(json.contains("\"version\":\"1.0\""));
    assert!(json.contains("\"params\""));

    // Verify Python's perspective
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["type"], "request");
    assert_eq!(parsed["method"], "process_audio");
    assert_eq!(parsed["version"], "1.0");
}

/// Test: approve_upgrade request format (Rust → Python)
/// Requirement: STT-REQ-007.1, Task 7.1.5 Phase 4
#[test]
fn test_approve_upgrade_request_format() {
    // Arrange: Create approve_upgrade request
    use meeting_minutes_automator_lib::ipc_protocol::PROTOCOL_VERSION;

    let request = IpcMessage::Request {
        id: "upgrade-789".to_string(),
        version: PROTOCOL_VERSION.to_string(),
        method: "approve_upgrade".to_string(),
        params: serde_json::json!({"target_model": "small"}),
    };

    // Act: Serialize to JSON (what Python will receive)
    let json = serde_json::to_string(&request).unwrap();

    // Assert: Python can parse this
    assert!(json.contains("\"type\":\"request\""));
    assert!(json.contains("\"method\":\"approve_upgrade\""));
    assert!(json.contains("\"params\""));
    assert!(json.contains("\"target_model\":\"small\""));

    // Verify Python's perspective
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["type"], "request");
    assert_eq!(parsed["method"], "approve_upgrade");
    assert_eq!(parsed["params"]["target_model"], "small");
}

/// Test: approve_upgrade response format (Python → Rust)
/// Requirement: STT-REQ-007.2, Task 7.1.5 Phase 6 (Fixed)
#[test]
fn test_approve_upgrade_response_format() {
    // Arrange: Simulate Python's approve_upgrade response (nested structure)
    let python_json = r#"{
        "id": "upgrade-123",
        "type": "response",
        "version": "1.0",
        "result": {
            "success": true,
            "old_model": "tiny",
            "new_model": "small"
        }
    }"#;

    // Act: Parse as IpcMessage (now works with result: serde_json::Value)
    let msg: IpcMessage = serde_json::from_str(python_json).unwrap();

    // Assert: Correctly parsed
    match msg {
        IpcMessage::Response {
            id,
            version,
            result,
        } => {
            assert_eq!(id, "upgrade-123");
            assert_eq!(version, "1.0");

            // Extract generic result fields
            assert_eq!(result["success"], true);
            assert_eq!(result["old_model"], "tiny");
            assert_eq!(result["new_model"], "small");
        }
        _ => panic!("Expected Response variant"),
    }
}

/// Test: approve_upgrade error format (Python → Rust)
/// Requirement: STT-REQ-007.5, Task 7.1.5 Phase 4
#[test]
fn test_approve_upgrade_error_format() {
    // Arrange: Simulate Python's approve_upgrade error (missing parameter)
    let python_json = r#"{
        "id": "upgrade-error",
        "type": "error",
        "version": "1.0",
        "errorCode": "MISSING_PARAMETER",
        "errorMessage": "Missing 'target_model' field in approve_upgrade request",
        "recoverable": true
    }"#;

    // Act: Parse as Rust IpcMessage
    let msg: IpcMessage = serde_json::from_str(python_json).unwrap();

    // Assert: Error format matches STT-REQ-007.5
    match msg {
        IpcMessage::Error {
            id,
            version,
            error_code,
            error_message,
            recoverable,
        } => {
            assert_eq!(id, "upgrade-error");
            assert_eq!(version, "1.0");
            assert_eq!(error_code, "MISSING_PARAMETER");
            assert_eq!(
                error_message,
                "Missing 'target_model' field in approve_upgrade request"
            );
            assert_eq!(recoverable, true);
        }
        _ => panic!("Expected Error variant"),
    }

    // Arrange: Simulate model load failure error
    let load_error_json = r#"{
        "id": "upgrade-load-fail",
        "type": "error",
        "version": "1.0",
        "errorCode": "MODEL_LOAD_ERROR",
        "errorMessage": "Failed to upgrade model: File not found",
        "recoverable": false
    }"#;

    // Act: Parse
    let msg2: IpcMessage = serde_json::from_str(load_error_json).unwrap();

    // Assert: Non-recoverable error
    match msg2 {
        IpcMessage::Error {
            error_code,
            recoverable,
            ..
        } => {
            assert_eq!(error_code, "MODEL_LOAD_ERROR");
            assert_eq!(recoverable, false);
        }
        _ => panic!("Expected Error variant"),
    }
}

/// Test: Event notification format (Python → Rust)
/// Requirement: STT-REQ-007.1, Task 7.1.5 Phase 5
#[test]
fn test_event_notification_format() {
    // Arrange: Simulate Python's model_change event
    let model_change_json = r#"{
        "type": "event",
        "version": "1.0",
        "eventType": "model_change",
        "data": {
            "old_model": "small",
            "new_model": "tiny",
            "reason": "cpu_high"
        }
    }"#;

    // Act: Parse as Rust IpcMessage
    let msg: IpcMessage = serde_json::from_str(model_change_json).unwrap();

    // Assert: Event format matches new protocol
    match msg {
        IpcMessage::Event {
            version,
            event_type,
            data,
        } => {
            assert_eq!(version, "1.0");
            assert_eq!(event_type, "model_change");
            assert_eq!(data["old_model"], "small");
            assert_eq!(data["new_model"], "tiny");
            assert_eq!(data["reason"], "cpu_high");
        }
        _ => panic!("Expected Event variant"),
    }

    // Arrange: Simulate Python's upgrade_proposal event
    let upgrade_proposal_json = r#"{
        "type": "event",
        "version": "1.0",
        "eventType": "upgrade_proposal",
        "data": {
            "current_model": "tiny",
            "proposed_model": "small",
            "message": "Resources have recovered. Upgrade to small?"
        }
    }"#;

    // Act: Parse
    let msg2: IpcMessage = serde_json::from_str(upgrade_proposal_json).unwrap();

    // Assert
    match msg2 {
        IpcMessage::Event {
            event_type, data, ..
        } => {
            assert_eq!(event_type, "upgrade_proposal");
            assert_eq!(data["current_model"], "tiny");
            assert_eq!(data["proposed_model"], "small");
            assert!(data["message"].as_str().unwrap().contains("Upgrade"));
        }
        _ => panic!("Expected Event variant"),
    }

    // Arrange: Simulate Python's recording_paused event
    let recording_paused_json = r#"{
        "type": "event",
        "version": "1.0",
        "eventType": "recording_paused",
        "data": {
            "reason": "insufficient_resources",
            "message": "System resources are critically low. Recording paused."
        }
    }"#;

    // Act: Parse
    let msg3: IpcMessage = serde_json::from_str(recording_paused_json).unwrap();

    // Assert
    match msg3 {
        IpcMessage::Event {
            event_type, data, ..
        } => {
            assert_eq!(event_type, "recording_paused");
            assert_eq!(data["reason"], "insufficient_resources");
            assert!(data["message"].as_str().unwrap().contains("critically low"));
        }
        _ => panic!("Expected Event variant"),
    }
}

/// Test: Event notification version defaults to "1.0" if omitted
/// Requirement: ADR-003 (backward compatibility)
#[test]
fn test_event_notification_version_default() {
    // Arrange: Event without version field (old format)
    let json_no_version = r#"{
        "type": "event",
        "eventType": "test_event",
        "data": {"key": "value"}
    }"#;

    // Act: Deserialize
    let msg: IpcMessage = serde_json::from_str(json_no_version).unwrap();

    // Assert: Version defaults to "1.0"
    assert_eq!(msg.version(), "1.0");
}

/// Test: Legacy StartProcessing converts to process_audio request with audio_data
/// Requirement: STT-REQ-007.1 (backward compatibility with data preservation)
#[test]
#[allow(deprecated)]
fn test_legacy_start_processing_conversion() {
    // Arrange: Legacy StartProcessing message with audio data
    let audio_data = vec![0u8, 1, 2, 3, 4, 5];
    let legacy = LegacyIpcMessage::StartProcessing {
        audio_data: audio_data.clone(),
    };

    // Act: Convert to new protocol
    let new_format = legacy.to_protocol_message();

    // Assert: Converted to Request with method="process_audio" (NOT "start_processing")
    let json = serde_json::to_string(&new_format).unwrap();
    let parsed: IpcMessage = serde_json::from_str(&json).unwrap();

    match parsed {
        IpcMessage::Request {
            id,
            version,
            method,
            ref params,
        } => {
            assert_eq!(id, "legacy-request");
            assert_eq!(version, "1.0");
            assert_eq!(method, "process_audio"); // CRITICAL: Must be process_audio, not start_processing
            assert!(params.is_object());

            // CRITICAL: Verify audio_data is preserved
            let audio_in_params = params["audio_data"].as_array().unwrap();
            assert_eq!(audio_in_params.len(), 6);
            assert_eq!(audio_in_params[0].as_u64().unwrap(), 0);
            assert_eq!(audio_in_params[5].as_u64().unwrap(), 5);
        }
        _ => panic!("Expected Request variant"),
    }

    // Assert: Python can parse this as process_audio
    assert!(json.contains("\"type\":\"request\""));
    assert!(json.contains("\"method\":\"process_audio\""));
    assert!(json.contains("\"audio_data\""));
}

/// Test: Legacy StopProcessing converts to new format request
/// Requirement: STT-REQ-007.1 (backward compatibility)
#[test]
#[allow(deprecated)]
fn test_legacy_stop_processing_conversion() {
    // Arrange: Legacy StopProcessing message
    let legacy = LegacyIpcMessage::StopProcessing;

    // Act: Convert to new protocol
    let new_format = legacy.to_protocol_message();

    // Assert: Converted to Request with method="stop_processing"
    let json = serde_json::to_string(&new_format).unwrap();
    let parsed: IpcMessage = serde_json::from_str(&json).unwrap();

    match parsed {
        IpcMessage::Request {
            id,
            version,
            method,
            ref params,
        } => {
            assert_eq!(id, "legacy-request");
            assert_eq!(version, "1.0");
            assert_eq!(method, "stop_processing");
            assert!(params.is_object());
        }
        _ => panic!("Expected Request variant"),
    }

    // Assert: Python can parse this
    assert!(json.contains("\"type\":\"request\""));
    assert!(json.contains("\"method\":\"stop_processing\""));
}

// ============================================================================
// Task 7.2: Version Compatibility Logic Tests
// Requirements: STT-REQ-007.6
// ============================================================================

/// Test: check_version_compatibility() detects major version mismatch
/// Requirement: STT-REQ-007.6 (Major version: error + reject)
#[test]
fn test_version_check_major_mismatch() {
    // Act: Check version 2.0 against 1.0
    let result = check_version_compatibility("2.0", "1.0");

    // Assert: MajorMismatch is returned
    match result {
        VersionCompatibility::MajorMismatch { received, expected } => {
            assert_eq!(received, "2.0");
            assert_eq!(expected, "1.0");
        }
        _ => panic!("Expected MajorMismatch, got {:?}", result),
    }
}

/// Test: check_version_compatibility() detects minor version mismatch
/// Requirement: STT-REQ-007.6 (Minor version: warning + backward compat)
#[test]
fn test_version_check_minor_mismatch() {
    // Act: Check version 1.1 against 1.0
    let result = check_version_compatibility("1.1", "1.0");

    // Assert: MinorMismatch is returned
    match result {
        VersionCompatibility::MinorMismatch { received, expected } => {
            assert_eq!(received, "1.1");
            assert_eq!(expected, "1.0");
        }
        _ => panic!("Expected MinorMismatch, got {:?}", result),
    }
}

/// Test: check_version_compatibility() allows patch version difference
/// Requirement: STT-REQ-007.6 (Patch version: info log only, continue normally)
#[test]
fn test_version_check_patch_compatible() {
    // Act: Check version 1.0.2 against 1.0.1
    let result = check_version_compatibility("1.0.2", "1.0.1");

    // Assert: Compatible is returned (patch difference ignored)
    assert_eq!(result, VersionCompatibility::Compatible);
}

/// Test: check_version_compatibility() detects malformed version
/// Requirement: STT-REQ-007.6 (Defensive error handling)
#[test]
fn test_version_check_malformed() {
    // Act: Check malformed version strings
    let result1 = check_version_compatibility("invalid", "1.0");
    let result2 = check_version_compatibility("1.x", "1.0");
    let result3 = check_version_compatibility("", "1.0");

    // Assert: Malformed is returned
    match result1 {
        VersionCompatibility::Malformed { received } => {
            assert_eq!(received, "invalid");
        }
        _ => panic!("Expected Malformed for 'invalid'"),
    }

    match result2 {
        VersionCompatibility::Malformed { received } => {
            assert_eq!(received, "1.x");
        }
        _ => panic!("Expected Malformed for '1.x'"),
    }

    match result3 {
        VersionCompatibility::Malformed { received } => {
            assert_eq!(received, "");
        }
        _ => panic!("Expected Malformed for empty string"),
    }
}

/// Test: IpcMessage.check_version_compatibility() method integration
/// Requirement: STT-REQ-007.6
#[test]
fn test_ipc_message_version_check_integration() {
    // Arrange: Create message with version 2.0 (major mismatch)
    let msg = IpcMessage::Response {
        id: "test-001".to_string(),
        version: "2.0".to_string(),
        result: serde_json::json!({"text": "test", "is_final": true}),
    };

    // Act: Check compatibility
    let result = msg.check_version_compatibility();

    // Assert: MajorMismatch is detected
    match result {
        VersionCompatibility::MajorMismatch { received, expected } => {
            assert_eq!(received, "2.0");
            assert_eq!(expected, PROTOCOL_VERSION);
        }
        _ => panic!("Expected MajorMismatch"),
    }
}

// ============================================================================
// Task 7.2: meeting-minutes-core (Fake) Compatibility Tests
// Requirements: STT-REQ-007.3
// ============================================================================

/// Test: Fake implementation ignores unknown fields (MVP0 → MVP1 migration)
/// Requirement: STT-REQ-007.3 (Fake SHALL ignore unknown fields, use only 'text')
#[test]
fn test_fake_implementation_compatibility() {
    // Arrange: MVP1 sends extended TranscriptionResult with new fields
    let mvp1_json = r#"{
        "type": "response",
        "id": "test-mvp1-001",
        "version": "1.0",
        "result": {
            "text": "こんにちは、世界",
            "is_final": true,
            "confidence": 0.95,
            "language": "ja",
            "processing_time_ms": 450,
            "model_size": "small"
        }
    }"#;

    // Act: MVP0 (Fake) parses this as IpcMessage
    let msg: IpcMessage = serde_json::from_str(mvp1_json).unwrap();

    // Assert: MVP0 successfully parses and extracts only 'text'
    match msg {
        IpcMessage::Response { result, .. } => {
            let transcription: TranscriptionResult = serde_json::from_value(result).unwrap();

            // MVP0 (Fake) uses only text field
            assert_eq!(transcription.text, "こんにちは、世界");

            // MVP0 ignores extended fields (they are Option<T> with #[serde(default)])
            // Verification: These fields may or may not be present in MVP0
            // The important point is that MVP0 doesn't crash
        }
        _ => panic!("Expected Response variant for fake compatibility"),
    }
}

/// Test: MVP0 minimal response is accepted by MVP1
/// Requirement: STT-REQ-007.3 (Backward compatibility)
#[test]
fn test_mvp0_minimal_response_accepted_by_mvp1() {
    // Arrange: MVP0 (Fake) sends minimal response (text + is_final only)
    let mvp0_json = r#"{
        "type": "response",
        "id": "test-mvp0-001",
        "version": "1.0",
        "result": {
            "text": "Hello from MVP0",
            "is_final": true
        }
    }"#;

    // Act: MVP1 parses this
    let msg: IpcMessage = serde_json::from_str(mvp0_json).unwrap();

    // Assert: MVP1 accepts minimal response
    match msg {
        IpcMessage::Response { result, .. } => {
            let transcription: TranscriptionResult = serde_json::from_value(result).unwrap();

            assert_eq!(transcription.text, "Hello from MVP0");
            assert_eq!(transcription.is_final, true);

            // Extended fields are None (backward compatibility)
            assert_eq!(transcription.confidence, None);
            assert_eq!(transcription.language, None);
            assert_eq!(transcription.processing_time_ms, None);
            assert_eq!(transcription.model_size, None);
        }
        _ => panic!("Expected Response variant for MVP0 minimal"),
    }
}

/// Test: Legacy IpcMessage can coexist with new protocol
/// Requirement: STT-REQ-007.1 (Migration strategy)
#[test]
fn test_legacy_and_new_protocol_coexistence() {
    // Arrange: Create both legacy and new format messages
    let legacy = LegacyIpcMessage::TranscriptionResult {
        text: "Legacy format".to_string(),
        timestamp: 123456,
    };

    let new = IpcMessage::Response {
        id: "test-new-001".to_string(),
        version: PROTOCOL_VERSION.to_string(),
        result: serde_json::json!({
            "text": "New format",
            "is_final": true
        }),
    };

    // Act: Convert legacy to new
    let legacy_as_new = legacy.to_protocol_message();

    // Assert: Both formats can be processed
    match legacy_as_new {
        IpcMessage::Response { result, .. } => {
            let transcription: TranscriptionResult = serde_json::from_value(result).unwrap();
            assert_eq!(transcription.text, "Legacy format");
        }
        _ => panic!("Expected Response variant for legacy conversion"),
    }

    match new {
        IpcMessage::Response { result, .. } => {
            let transcription: TranscriptionResult = serde_json::from_value(result).unwrap();
            assert_eq!(transcription.text, "New format");
        }
        _ => panic!("Expected Response variant for new protocol"),
    }
}
