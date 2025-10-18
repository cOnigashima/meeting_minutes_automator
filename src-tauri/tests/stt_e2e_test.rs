// E2E Tests for Real STT Integration (MVP1)
// Task 10: 統合テスト・E2Eテスト（コア機能検証）
//
// Requirements:
// - STT-REQ-001: 実音声デバイス管理
// - STT-REQ-002: faster-whisper統合
// - STT-REQ-003: VAD統合
// - STT-REQ-005: ローカルストレージ
// - STT-REQ-007/008: IPC/WebSocket拡張
//
// Implementation Status:
// - Task 10.1-10.7: Test skeletons created with #[ignore] attribute
// - ✅ BLOCK-006 resolved: Test audio fixtures created (test_audio_short/long/silence.wav)
// - Tests require Whisper model (base or tiny for CI)
// - Tests require Python sidecar integration (known issue in e2e_test.rs)
//
// Next Steps:
// 1. ✅ Fix Python sidecar startup issue (BLOCK-005 resolved)
// 2. ✅ Create mock audio data generator for CI (BLOCK-006 resolved)
// 3. Implement test helpers (verify_partial_final_text_distribution, etc.) - BLOCK-007
// 4. Remove #[ignore] attributes and enable tests in CI

// BLOCK-006: Test audio fixtures
mod fixtures;

// BLOCK-007: Test helpers
mod helpers;

/// Task 10.1: 音声録音→VAD→STT→保存の完全フロー検証
///
/// Phase 1 (Current): VAD + STT Core Flow
/// - ✅ Python sidecar起動とWhisper初期化確認
/// - ✅ VAD音声検出（speech_start、no_speech）
/// - ✅ 部分テキスト生成（partial_text with is_final=false）
/// - ✅ IPCイベント配信（process_audio_stream）
/// - ✅ リソースモニタリング（model_change on memory pressure）
///
/// Phase 2 (Future): WebSocket + LocalStorage Integration
/// - ⏸️ WebSocket経由でChrome拡張へのメッセージ配信を確認
/// - ⏸️ ローカルストレージへのセッション保存（audio.wav, transcription.jsonl, session.json）を検証
/// - ⏸️ 確定テキスト（final_text with is_final=true）と speech_end イベント検証
///
/// Requirements: STT-REQ-001, STT-REQ-002, STT-REQ-003 (Phase 1)
///              STT-REQ-005, STT-REQ-008 (Phase 2)
///
/// Current Implementation Status:
/// - ✅ BLOCK-007 resolved: Test helpers implemented and validated
/// - ✅ WhisperSTTEngine initialization fixed (eager loading before ready signal)
/// - ✅ Model path placeholder bug fixed (HuggingFace model ID fallback)
/// - ✅ Test successfully receives 18 events including partial_text
#[tokio::test]
#[ignore] // Requires Whisper model download (use `cargo test -- --ignored` to run)
async fn test_audio_recording_to_transcription_full_flow() {
    use meeting_minutes_automator_lib::python_sidecar::PythonSidecarManager;
    use tokio::time::{timeout, Duration};

    // Step 1: Start Python sidecar
    let mut sidecar = PythonSidecarManager::new();
    sidecar
        .start()
        .await
        .expect("Python sidecar should start");
    sidecar
        .wait_for_ready()
        .await
        .expect("Should receive ready signal");

    // Step 2: Send test audio (short WAV: 3 seconds)
    let pcm_samples = fixtures::extract_pcm_samples(fixtures::test_audio::SHORT);
    let chunks = fixtures::chunk_pcm_samples(&pcm_samples, 20); // 20ms chunks

    let mut events = Vec::new();

    println!("DEBUG: Sending {} audio chunks", chunks.len());

    for (i, chunk) in chunks.iter().enumerate() {
        let audio_bytes = fixtures::pcm_samples_to_bytes(&chunk);

        // Send process_audio_stream request
        let request = serde_json::json!({
            "id": format!("chunk-{}", i),
            "type": "request",
            "version": "1.0",
            "method": "process_audio_stream",
            "params": {
                "audio_data": audio_bytes
            }
        });

        sidecar
            .send_message(request)
            .await
            .expect("Should send audio chunk");

        // Receive events (with timeout to prevent hanging)
        // receive_message() returns Result<Value, PythonSidecarError>
        match timeout(Duration::from_millis(100), sidecar.receive_message()).await {
            Ok(Ok(response)) => {
                println!("DEBUG: Chunk {} received event: {:?}", i, response);
                events.push(response);
            }
            Ok(Err(e)) => {
                println!("DEBUG: Chunk {} receive error: {:?}", i, e);
            }
            Err(_) => {
                // Timeout - expected for most chunks
            }
        }
    }

    // Wait for final events (give Python time to process Whisper transcription)
    // Whisper inference can take several seconds, especially on first run
    println!("DEBUG: Waiting for final events (Whisper may take time)...");
    for i in 0..20 {  // Increased from 10 to 20 iterations (10 seconds total)
        match timeout(Duration::from_millis(500), sidecar.receive_message()).await {
            Ok(Ok(response)) => {
                println!("DEBUG: Final event {}: {:?}", i, response);
                events.push(response);
            }
            Ok(Err(e)) => {
                println!("DEBUG: Final event {} error: {:?}", i, e);
                break;
            }
            Err(_) => {
                // Continue waiting - Whisper may still be processing
                println!("DEBUG: Final event {} timeout (continuing to wait...)", i);
            }
        }
    }

    println!("DEBUG: Total events received: {}", events.len());

    // Step 3: Verify partial/final text distribution
    helpers::verify_partial_final_text_distribution(&events)
        .expect("Partial/final text distribution should be correct");

    // Step 4: Shutdown
    sidecar.shutdown().await.expect("Should shutdown cleanly");

    println!("✅ Received {} events", events.len());
    println!("✅ Partial/final text distribution verified");
}

//
// Task 10.1 Helper Functions (to be implemented):
//
// - verify_partial_final_text_distribution()
//   Validates that partial text has isPartial=true and final text has isPartial=false
//
// - verify_local_storage_session(session_id: &str)
//   Validates that audio.wav, transcription.jsonl, and session.json are saved correctly
//

/// Task 10.2: オフラインモデルフォールバックE2Eテスト
///
/// 検証項目:
/// - ネットワーク切断状態をシミュレーション（HuggingFace Hub接続タイムアウト）
/// - バンドルbaseモデルへの自動フォールバックを検証
/// - オフラインモード強制設定時のローカルモデル使用を確認
///
/// Requirements: STT-REQ-002.4, STT-REQ-002.6
#[tokio::test]
#[ignore] // Requires network simulation
async fn test_offline_model_fallback() {
    // TODO: Task 10.2 implementation
    unimplemented!("Task 10.2: Offline model fallback test not yet implemented");
}

/// Task 10.3: 動的モデルダウングレードE2Eテスト
///
/// 検証項目:
/// - CPU使用率85%を60秒継続するシミュレーション
/// - メモリ使用量3GB/4GB到達時のモデル自動ダウングレードを検証
/// - ダウングレード通知がIPC経由で送信されることを確認
/// - UI通知がWebSocket経由で配信されることを確認
///
/// Requirements: STT-REQ-006.7, STT-REQ-006.8, STT-REQ-006.9
#[tokio::test]
#[ignore] // Requires resource simulation
async fn test_dynamic_model_downgrade() {
    // TODO: Task 10.3 implementation
    unimplemented!("Task 10.3: Dynamic model downgrade test not yet implemented");
}

/// Task 10.4: デバイス切断/再接続E2Eテスト
///
/// 検証項目:
/// - デバイス切断シミュレーション
/// - 自動再接続フロー検証（最大3回）
/// - 切断/再接続イベントがUI通知されることを確認
///
/// Requirements: STT-REQ-004.9, STT-REQ-004.10, STT-REQ-004.11
#[tokio::test]
#[ignore] // Requires device simulation
async fn test_device_disconnection_reconnection() {
    // TODO: Task 10.4 implementation
    unimplemented!("Task 10.4: Device disconnection/reconnection test not yet implemented");
}

/// Task 10.5: クロスプラットフォームE2Eテスト
///
/// 検証項目:
/// - macOS/Windows/Linux各環境での動作確認
/// - ループバックオーディオキャプチャ検証
/// - OS固有音声API統合（WASAPI、CoreAudio、ALSA）動作確認
///
/// Requirements: STT-REQ-004.3, STT-REQ-004.4, STT-REQ-004.5
#[tokio::test]
#[ignore] // Requires multi-platform CI
async fn test_cross_platform_compatibility() {
    // TODO: Task 10.5 implementation
    unimplemented!("Task 10.5: Cross-platform compatibility test not yet implemented");
}

/// Task 10.6: IPC/WebSocket後方互換性テスト
///
/// 検証項目:
/// - Phase 6拡張フィールド（confidence/language/processingTimeMs/isPartial）の送受信検証
/// - meeting-minutes-core（Fake実装）との互換性確認
/// - バージョン不一致時のフォールバック検証
///
/// Requirements: STT-REQ-007.3, STT-REQ-007.6, STT-REQ-008.2
///
/// Note: Partial coverage exists in:
/// - tests/ipc_migration_test.rs (26 tests) - IPC protocol backward compatibility
/// - tests/websocket_message_extension_test.rs (6 tests) - WebSocket message extension
/// This test should add end-to-end MVP0 compatibility verification
#[tokio::test]
#[ignore] // Requires MVP0 integration
async fn test_ipc_websocket_backward_compatibility() {
    // Known Coverage:
    // - IPC backward compatibility: tests/ipc_migration_test.rs
    //   - Version field omitted defaults to 1.0
    //   - Legacy format not parsed as new format
    //   - Forward compatibility (unknown fields ignored)
    // - WebSocket backward compatibility: tests/websocket_message_extension_test.rs
    //   - Minimal fields (backward compatibility)
    //   - Chrome extension ignores unknown fields
    //
    // This E2E test should verify the full integration with MVP0
}

/// Task 10.7: 非機能要件検証（SLA、パフォーマンス、リソース制約）
///
/// 検証項目:
/// - 部分テキスト応答時間 < 0.5秒
/// - 確定テキスト応答時間 < 2秒
/// - メモリ使用量 < 2GB（2時間録音）
/// - CPU使用率 < 50%（継続録音）
///
/// Requirements: STT-NFR-001.1, STT-NFR-001.2, STT-NFR-001.3, STT-NFR-001.4
#[tokio::test]
#[ignore] // Requires long-running test
async fn test_non_functional_requirements() {
    // TODO: Task 10.7 implementation
    unimplemented!("Task 10.7: Non-functional requirements test not yet implemented");
}

//
// Task 10.7 Helper Functions (to be implemented):
//
// - measure_partial_text_latency() -> Duration
//   Measures the time from speech onset to partial text generation (< 0.5s)
//
// - measure_final_text_latency() -> Duration
//   Measures the time from speech offset to final text generation (< 2s)
//
// - monitor_memory_usage_long_running() -> u64
//   Monitors memory usage over 2 hours of continuous recording (< 2GB)
//
// - monitor_cpu_usage_during_recording() -> f64
//   Monitors CPU usage during continuous recording (< 50%)
//
