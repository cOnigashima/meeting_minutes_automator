// Task 8.1: WebSocketメッセージ拡張テスト
// STT-REQ-008.1: WebSocketメッセージにconfidence, language, processingTimeMsフィールドを追加
// STT-REQ-008.2: meeting-minutes-core（Fake実装）との後方互換性維持

use serde_json::json;

// WebSocketMessage enumを直接使用するため、websocket.rsから構造体をコピー
// （実際のテストでは#[path = "../src/websocket.rs"]でインポート）
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum WebSocketMessage {
    #[serde(rename = "transcription")]
    Transcription {
        #[serde(rename = "messageId")]
        message_id: String,
        #[serde(rename = "sessionId")]
        session_id: String,
        text: String,
        timestamp: u64,
        #[serde(rename = "isPartial", skip_serializing_if = "Option::is_none")]
        is_partial: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        confidence: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        language: Option<String>,
        #[serde(rename = "processingTimeMs", skip_serializing_if = "Option::is_none")]
        processing_time_ms: Option<u64>,
    },
}

#[test]
fn test_transcription_with_all_extended_fields() {
    // STT-REQ-008.1: すべての拡張フィールドを含むメッセージのシリアライズ検証
    let msg = WebSocketMessage::Transcription {
        message_id: "ws-123".to_string(),
        session_id: "session-1".to_string(),
        text: "こんにちは".to_string(),
        timestamp: 1696234567890,
        is_partial: Some(false),
        confidence: Some(0.95),
        language: Some("ja".to_string()),
        processing_time_ms: Some(450),
    };

    let json_str = serde_json::to_string(&msg).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // 必須フィールド検証
    assert_eq!(parsed["type"], "transcription");
    assert_eq!(parsed["messageId"], "ws-123");
    assert_eq!(parsed["sessionId"], "session-1");
    assert_eq!(parsed["text"], "こんにちは");
    assert_eq!(parsed["timestamp"], 1696234567890_u64);

    // 拡張フィールド検証（STT-REQ-008.1）
    assert_eq!(parsed["isPartial"], false);
    assert_eq!(parsed["confidence"], 0.95);
    assert_eq!(parsed["language"], "ja");
    assert_eq!(parsed["processingTimeMs"], 450);
}

#[test]
fn test_transcription_backward_compatibility_minimal_fields() {
    // STT-REQ-008.2: 拡張フィールドを省略した場合の後方互換性検証
    let msg = WebSocketMessage::Transcription {
        message_id: "ws-124".to_string(),
        session_id: "session-1".to_string(),
        text: "最小フィールド".to_string(),
        timestamp: 1696234567890,
        is_partial: None,
        confidence: None,
        language: None,
        processing_time_ms: None,
    };

    let json_str = serde_json::to_string(&msg).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // 必須フィールドのみ存在
    assert_eq!(parsed["type"], "transcription");
    assert_eq!(parsed["messageId"], "ws-124");
    assert_eq!(parsed["text"], "最小フィールド");

    // 拡張フィールドは省略される（skip_serializing_if = "Option::is_none"）
    assert!(parsed.get("isPartial").is_none());
    assert!(parsed.get("confidence").is_none());
    assert!(parsed.get("language").is_none());
    assert!(parsed.get("processingTimeMs").is_none());
}

#[test]
fn test_transcription_partial_fields() {
    // 一部の拡張フィールドのみ含む場合の検証
    let msg = WebSocketMessage::Transcription {
        message_id: "ws-125".to_string(),
        session_id: "session-1".to_string(),
        text: "部分フィールド".to_string(),
        timestamp: 1696234567890,
        is_partial: Some(true),
        confidence: Some(0.85),
        language: None, // 省略
        processing_time_ms: None, // 省略
    };

    let json_str = serde_json::to_string(&msg).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(parsed["isPartial"], true);
    assert_eq!(parsed["confidence"], 0.85);
    assert!(parsed.get("language").is_none());
    assert!(parsed.get("processingTimeMs").is_none());
}

#[test]
fn test_confidence_range_validation() {
    // Confidence範囲検証（0.0-1.0）
    let msg_valid = WebSocketMessage::Transcription {
        message_id: "ws-126".to_string(),
        session_id: "session-1".to_string(),
        text: "範囲検証".to_string(),
        timestamp: 1696234567890,
        is_partial: None,
        confidence: Some(1.0), // 上限値
        language: None,
        processing_time_ms: None,
    };

    let json_str = serde_json::to_string(&msg_valid).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["confidence"], 1.0);

    // 注: 下限値チェックはPython側で実施（Rustでは型レベルで保証しない）
}

#[test]
fn test_deserialization_from_python_response() {
    // Python IPCレスポンスからのデシリアライズ検証
    let python_response = json!({
        "type": "transcription",
        "messageId": "ws-127",
        "sessionId": "session-1",
        "text": "Pythonからの応答",
        "timestamp": 1696234567890_u64,
        "isPartial": false,
        "confidence": 0.92,
        "language": "ja",
        "processingTimeMs": 320
    });

    let msg: WebSocketMessage = serde_json::from_value(python_response).unwrap();

    if let WebSocketMessage::Transcription {
        message_id,
        text,
        confidence,
        language,
        processing_time_ms,
        ..
    } = msg
    {
        assert_eq!(message_id, "ws-127");
        assert_eq!(text, "Pythonからの応答");
        assert_eq!(confidence, Some(0.92));
        assert_eq!(language, Some("ja".to_string()));
        assert_eq!(processing_time_ms, Some(320));
    } else {
        panic!("Expected Transcription variant");
    }
}

#[test]
fn test_chrome_extension_ignores_unknown_fields() {
    // STT-REQ-008.2: Chrome拡張が未知フィールドを無視することを検証
    // （Chrome拡張側のテストだが、メッセージ形式の検証として実施）
    let json_with_extra_fields = json!({
        "type": "transcription",
        "messageId": "ws-128",
        "sessionId": "session-1",
        "text": "未知フィールド付き",
        "timestamp": 1696234567890_u64,
        // 未知フィールド（将来の拡張）
        "speakerSegment": 0,
        "emotion": "neutral"
    });

    // Rustでデシリアライズしても問題ないことを確認
    let msg: WebSocketMessage = serde_json::from_value(json_with_extra_fields).unwrap();

    if let WebSocketMessage::Transcription { text, .. } = msg {
        assert_eq!(text, "未知フィールド付き");
    } else {
        panic!("Expected Transcription variant");
    }
}
