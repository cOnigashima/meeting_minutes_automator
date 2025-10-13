# ADR-009: Sender/Receiver Concurrent Architecture（IPC Deadlock根本解決）

## Status
❌ **Rejected (2025-10-13)** - Superseded by ADR-011 + ADR-012

**Rejection Reason**: 技術的検証により、以下の**2つの構造的欠陥**が発見されました：

1. **Mutex共有によるシリアライゼーション（P0）**: `Arc<Mutex<PythonSidecarManager>>`の共有により、Sender/Receiver並行実行が実質シリアライズされる。真の並行性が失われ、ADR-008の構造的デッドロックが解消されていない。
2. **blocking_send()によるCPALストリーム停止（P0）**: オーディオコールバックで`blocking_send()`を使用。Python異常時（ハング/クラッシュ）にバッファ満杯で最大2秒ブロック → CPALのOSバッファ（128ms）がオーバーラン → ストリーム停止。

**解決策**:
- ADR-011（IPC Stdin/Stdout Mutex分離）で問題1を解決
- ADR-012（Audio Callback Backpressure再設計）で問題2を解決

## Context

### ADR-008の致命的欠陥

ADR-008 v1.1（Dedicated Session Task）の外部レビューにより、以下の**3つの致命的欠陥**が発見されました：

#### 欠陥1: 構造的デッドロック（P0 Critical）

```rust
// ADR-008の実装（問題あり）
while let Some(audio_data) = frame_rx.recv().await {  // 1フレーム受信
    // Send frame to Python
    sidecar.send_message(request).await;

    // Block until speech_end/no_speech
    loop {
        let event = sidecar.receive_message().await;  // ← デッドロック！
        if event_type == "speech_end" || "no_speech" {
            break;  // 次フレームへ
        }
    }
}
```

**問題点**:
- 1フレーム送信 → `speech_end`待ち → 次フレーム送信 の順序
- Whisperは**複数フレームを見てから**`speech_end`を判定
- 次フレームが送信されない → Pythonは`speech_end`を出せない → Rustは永久待ち
- **これは既存実装と全く同じデッドロック！**

#### 欠陥2: Python偽`no_speech`検出（P1 High）

```python
# python-stt/main.py:294-418（問題あり）
speech_detected = False

for frame in frames:
    result = await self.pipeline.process_audio_frame_with_partial(frame)
    if result:  # イベント発行時のみtrueに
        speech_detected = True

# 問題: イベント発行の有無だけで判定
if not speech_detected:
    await self.ipc.send_message({'eventType': 'no_speech'})  # 誤検知！
```

**問題シナリオ**:
1. ユーザーが話し続けている
2. 前回リクエストで`partial_text`送信後、`_frame_count_since_partial`リセット
3. 次リクエストで30-80フレーム処理中、まだ次の`partial_text`なし
4. `result`が`None` → `speech_detected = False`
5. **`no_speech`を誤送信** → Rust側が発話中なのに無音判定

**正しい判定**: VAD状態（`vad.is_in_speech()`）を見るべき。

#### 欠陥3: Backpressure Frameドロップ

```rust
// ADR-008の実装（品質劣化）
frame_tx.try_send(audio_data)  // バッファフルで即ドロップ
```

**問題点**:
- 10msフレームの欠落が音声ストリーム破損を招く
- 文字起こし精度低下（garbled transcripts）
- ステークホルダー承認なしの機能劣化

---

## Decision

**採用アーキテクチャ**: Sender/Receiver Concurrent Tasks

### Core Principle

**送信と受信を完全に並行実行する**:
- Sender Task: フレームを連続的にPythonへ送信（ブロックなし）
- Receiver Task: Pythonからイベントを連続的に受信（ブロックなし）
- 両タスクは独立して動作、mutexは最小スコープで共有

### Architecture Diagram

```
Audio Callback (10ms interval)
      |
      v
  frame_tx (mpsc::Sender)
      |
      v
  frame_rx (mpsc::Receiver)
      |
      v
  +----------------------------------+
  |  Recording Session Task          |
  |                                  |
  |  ┌────────────────────────────┐ |
  |  │  Sender Task               │ |
  |  │  (Independent)             │ |
  |  │                            │ |
  |  │  loop {                    │ |
  |  │    frame = frame_rx.recv() │ |
  |  │    python.send(frame)      │ |  ← Mutex scope: send only
  |  │  }                         │ |
  |  └────────────────────────────┘ |
  |                                  |
  |  ┌────────────────────────────┐ |
  |  │  Receiver Task             │ |
  |  │  (Independent)             │ |
  |  │                            │ |
  |  │  loop {                    │ |
  |  │    event = python.recv()   │ |  ← Mutex scope: recv only
  |  │    broadcast(event)        │ |
  |  │  }                         │ |
  |  └────────────────────────────┘ |
  +----------------------------------+
```

**Key Innovation**: Sender/Receiverは互いに待たない。Pythonは常に次フレームを受信でき、Whisperは必要なだけフレームを蓄積してから`speech_end`を送信できる。

---

## Implementation

### Phase 1: Concurrent Sender/Receiver (3-4時間)

#### 1.1 Session Task Spawn

```rust
fn spawn_recording_session_task(
    python_sidecar: Arc<tokio::Mutex<PythonSidecarManager>>,
    websocket_server: Arc<tokio::Mutex<WebSocketServer>>,
    mut frame_rx: mpsc::Receiver<Vec<u8>>,
    event_tx: broadcast::Sender<serde_json::Value>,
) -> SessionHandle {
    let python_sender = Arc::clone(&python_sidecar);
    let python_receiver = Arc::clone(&python_sidecar);

    // Metrics shared between tasks
    let metrics = Arc::new(SessionMetrics::new());
    let metrics_send = Arc::clone(&metrics);
    let metrics_recv = Arc::clone(&metrics);

    // Sender Task: Continuously send frames
    let sender_handle = tokio::spawn(async move {
        while let Some(audio_data) = frame_rx.recv().await {
            let request = ProtocolMessage::Request {
                id: format!("audio-{}", SystemTime::now().as_millis()),
                version: PROTOCOL_VERSION.to_string(),
                method: "process_audio_stream".to_string(),
                params: serde_json::json!({ "audio_data": audio_data }),
            };

            // Send with minimal mutex scope
            {
                let mut sidecar = python_sender.lock().await;
                if let Err(e) = sidecar.send_message(serde_json::to_value(&request).unwrap()).await {
                    eprintln!("[Sender] Send error: {:?}", e);
                    metrics_send.ipc_errors.fetch_add(1, Ordering::Relaxed);
                    // Continue sending next frame (don't break)
                }
                // Mutex dropped immediately
            }

            metrics_send.frames_sent.fetch_add(1, Ordering::Relaxed);
        }

        eprintln!("[Sender] Frame channel closed, exiting");
    });

    // Receiver Task: Continuously receive events
    let receiver_handle = tokio::spawn(async move {
        let mut error_count = 0;

        loop {
            // Receive with minimal mutex scope
            let event_result = {
                let mut sidecar = python_receiver.lock().await;
                sidecar.receive_message().await
                // Mutex dropped immediately
            };

            match event_result {
                Ok(event) => {
                    error_count = 0;  // Reset on success
                    metrics_recv.events_received.fetch_add(1, Ordering::Relaxed);

                    // Parse IPC message with error handling (ADR-008 v1.1)
                    match serde_json::from_value::<ProtocolMessage>(event.clone()) {
                        Ok(msg) => {
                            // Valid message - broadcast to subscribers
                            let _ = event_tx.send(event.clone());

                            // Forward to WebSocket
                            match msg {
                                ProtocolMessage::Event { event_type, data, .. } => {
                                    if event_type == "final_text" {
                                        let text = data.get("text").and_then(|v| v.as_str()).unwrap_or("");
                                        let ws_msg = WebSocketMessage::Transcription {
                                            message_id: format!("ws-{}", SystemTime::now().as_millis()),
                                            session_id: "session-1".to_string(),
                                            text: text.to_string(),
                                            timestamp: SystemTime::now()
                                                .duration_since(UNIX_EPOCH)
                                                .unwrap()
                                                .as_millis() as u64,
                                        };

                                        let ws_server = websocket_server.lock().await;
                                        let _ = ws_server.broadcast(ws_msg).await;
                                    }

                                    // CRITICAL: Do NOT break on speech_end!
                                    // Continue receiving events for next chunks
                                }
                                ProtocolMessage::Error { error_message, .. } => {
                                    eprintln!("[Receiver] Python error: {}", error_message);
                                    // Log but continue (don't break)
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            eprintln!("[Receiver] JSON parse error: {:?}", e);
                            metrics_recv.parse_errors.fetch_add(1, Ordering::Relaxed);
                            // DO NOT broadcast corrupted event
                            // Continue to next event (don't break)
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[Receiver] IPC error: {:?}", e);
                    metrics_recv.ipc_errors.fetch_add(1, Ordering::Relaxed);

                    // Exponential backoff
                    error_count += 1;
                    let backoff = Duration::from_millis(100 * 2u64.pow(error_count.min(5)));
                    tokio::time::sleep(backoff).await;

                    if error_count > 10 {
                        eprintln!("[Receiver] Too many errors, terminating");
                        break;
                    }
                }
            }
        }

        eprintln!("[Receiver] Exiting due to errors");
    });

    SessionHandle {
        sender_handle,
        receiver_handle,
        metrics,
    }
}

struct SessionHandle {
    sender_handle: tokio::task::JoinHandle<()>,
    receiver_handle: tokio::task::JoinHandle<()>,
    metrics: Arc<SessionMetrics>,
}
```

**重要な設計判断**:
1. **Receiverは無限ループ**: `speech_end`でbreakしない（次チャンクのイベントも受信）
2. **Mutexスコープ最小化**: 送信/受信の瞬間だけlock、即解放
3. **エラー耐性**: 送信失敗しても次フレーム継続、受信失敗はExponential backoff

#### 1.2 Audio Callback（変更なし）

```rust
device.start_with_callback(move |audio_data| {
    // Bounded buffer with blocking on full (ADR-009改善)
    if let Err(e) = frame_tx.blocking_send(audio_data) {
        eprintln!("[Audio Callback] Failed to send frame: {:?}", e);
        metrics.frames_dropped.fetch_add(1, Ordering::Relaxed);
    }
});
```

**Backpressure改善**:
- `try_send()` → `blocking_send()`: バッファフル時はオーディオスレッドが待つ
- バッファサイズ増加: 100 → 200フレーム（20秒分）
- VAD無音検出時のみドロップ許容（将来実装）

#### 1.3 Graceful Shutdown

```rust
// stop_recording()
pub async fn stop_recording(session_handle: SessionHandle) {
    // 1. Close frame channel (signals sender to stop)
    drop(frame_tx);

    // 2. Wait for sender to finish sending all queued frames
    let _ = session_handle.sender_handle.await;

    // 3. Abort receiver (no more events expected)
    session_handle.receiver_handle.abort();

    // 4. Report final metrics
    session_handle.metrics.report();
}
```

---

### Phase 2: Python VAD-Based `no_speech` Detection (1-2時間)

#### 2.1 AudioPipeline API拡張

```python
# python-stt/stt_engine/audio_pipeline.py
class AudioPipeline:
    def is_in_speech(self) -> bool:
        """Check if currently in speech state (VAD detected voice)."""
        return self._in_speech_state

    def has_buffered_speech(self) -> bool:
        """Check if there are speech frames buffered for STT processing."""
        return len(self._speech_frames) > 0
```

#### 2.2 main.py修正

```python
# python-stt/main.py:408-425（修正版）
if not speech_detected:
    # CRITICAL: Check VAD state, not just event emission
    if not self.pipeline.is_in_speech() and not self.pipeline.has_buffered_speech():
        logger.debug(f"No speech detected (VAD confirmed silence) for {msg_id}")
        await self.ipc.send_message({
            'type': 'event',
            'version': '1.0',
            'eventType': 'no_speech',
            'data': {
                'requestId': msg_id,
                'timestamp': int(time.time() * 1000)
            }
        })
    else:
        # Speech in progress but no event yet - DO NOT send no_speech
        logger.debug(f"Speech in progress (no event yet) for {msg_id}")
        # No response sent - Rust receiver will keep waiting for next event
```

**重要**: 発話継続中は`no_speech`を送らない。Receiver Taskが次のイベントを待ち続ける。

---

### Phase 3: E2E Testing (3-4時間)

#### 3.1 Long Utterance Test

```rust
#[tokio::test]
async fn test_long_utterance_no_deadlock() {
    let (frame_tx, frame_rx) = mpsc::channel(200);
    let (event_tx, mut event_rx) = broadcast::channel(1000);

    // Spawn session task
    let session = spawn_recording_session_task(
        mock_python_sidecar(),
        mock_websocket_server(),
        frame_rx,
        event_tx,
    );

    // Send 1000 frames (100 seconds of audio)
    for i in 0..1000 {
        frame_tx.send(vec![i as u8; 320]).await.unwrap();
    }

    // Mock Python: Send events every 100 frames (10s)
    // speech_start → partial_text (10s) → partial_text (20s) → ... → final_text + speech_end (100s)

    // Verify all events received WITHOUT timeout
    let mut event_count = 0;
    while let Ok(event) = tokio::time::timeout(Duration::from_secs(5), event_rx.recv()).await {
        event_count += 1;
        if event_count >= 10 {  // Expected: 10 partial + 1 final + 1 speech_end
            break;
        }
    }

    assert_eq!(event_count, 12, "Should receive all events without deadlock");
}
```

#### 3.2 Concurrent Chunks Test

```rust
#[tokio::test]
async fn test_concurrent_100_chunks() {
    // Send 100 chunks rapidly (simulating 10s recording)
    for i in 0..100 {
        frame_tx.send(generate_speech_frame(i)).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Verify: All frames sent, all events received, no deadlock
    assert_eq!(metrics.frames_sent.load(Ordering::Relaxed), 100);
    assert!(metrics.events_received.load(Ordering::Relaxed) > 0);
}
```

#### 3.3 Python `no_speech` Accuracy Test

```python
# python-stt/tests/test_no_speech_detection.py
async def test_no_false_no_speech_during_utterance():
    handler = IPCHandler(mock_audio_pipeline_with_vad())

    # Simulate: User talking continuously
    # Request 1: Send partial_text
    await handler.handle_message({'method': 'process_audio_stream', 'params': {'audio_data': speech_frames_1}})
    # → Should send partial_text

    # Request 2: 30 frames, no new partial yet, BUT VAD still detects speech
    await handler.handle_message({'method': 'process_audio_stream', 'params': {'audio_data': speech_frames_2}})
    # → Should NOT send no_speech (VAD says still speaking)

    # Verify: no_speech was NOT sent
    assert 'no_speech' not in sent_events
```

---

## Consequences

### Advantages

1. **✅ デッドロック完全解消**: Sender/Receiver並行実行でWhisperが必要なだけフレーム蓄積可能
2. **✅ 偽no_speech解消**: VAD状態ベース判定で発話中の誤検知なし
3. **✅ Backpressure改善**: `blocking_send()`で無音以外のドロップなし
4. **✅ 既存テスト互換**: IPC protocol変更なし（メソッド名そのまま）

### Disadvantages

1. **⚠️ 実装複雑度増**: 2つの並行タスク管理
2. **⚠️ Mutex競合可能性**: Sender/Receiverが同時lock試行（ただしスコープ最小化で緩和）
3. **⚠️ Python API変更**: `is_in_speech()`, `has_buffered_speech()` 追加実装必要

### Risks

1. **Receiver無限ループ**: `speech_end`でbreakしないため、セッション終了をframe_rx閉鎖で制御
2. **イベント順序保証**: Sender/Receiver並行でもイベント順序は保証される（Pythonが順次送信）
3. **Mutex fairness**: tokio::MutexはFIFOでないが、スコープ最小化で問題なし

---

## Success Criteria

以下を全て満たした場合に成功とみなす:

1. **✅ 長時間発話テスト**: 100秒連続発話でデッドロックなし
2. **✅ 偽no_speech解消**: 発話中に`no_speech`誤送信なし
3. **✅ フレームdrop率**: < 5%（バッファサイズ200で達成）
4. **✅ 既存テスト合格**: Rust 26 + Python 143全パス
5. **✅ E2Eテスト**: 実Tauri app + Fake Python sidecarで10秒発話検証

---

## Comparison with ADR-008

| 項目 | ADR-008 v1.1 | ADR-009 |
|------|-------------|---------|
| **デッドロック** | ❌ 発生（1フレーム送信→speech_end待ち） | ✅ 解消（Sender/Receiver並行） |
| **偽no_speech** | ❌ 発生（イベント有無で判定） | ✅ 解消（VAD状態で判定） |
| **Backpressure** | ⚠️ ドロップ許容 | ✅ Blocking送信（ドロップ最小化） |
| **実装複雑度** | 低 | 中 |
| **IPC protocol変更** | なし | なし |
| **Python API変更** | なし | あり（VADメソッド追加） |
| **推奨** | ❌ **Rejected** | ✅ **Approved** |

---

## Migration from ADR-008

ADR-008は実装前に却下されたため、マイグレーション不要。直接ADR-009を実装する。

---

## Rollback Strategy

### Feature Flag

```rust
const USE_CONCURRENT_SENDER_RECEIVER: bool = true;

pub async fn start_recording() {
    if USE_CONCURRENT_SENDER_RECEIVER {
        start_recording_with_concurrent_tasks().await  // ADR-009
    } else {
        start_recording_legacy().await  // Current implementation
    }
}
```

### Gradual Rollout

1. Week 1: Internal testing（開発者のみ）
2. Week 2: Beta users (10%)
3. Week 3: Full rollout (100%)

### Rollback Trigger

- デッドロック発生率 > 0.1%
- フレームdrop率 > 5%
- 偽no_speech報告 > 3件

---

## References

- **ADR-008**: Rejected (2025-10-13) - 構造的デッドロック欠陥のため
- **STT-REQ-007**: Event Stream Protocol (IPC Protocol Extension)
- **外部レビュー指摘（2025-10-13）**: Sender/Receiver分離推奨
- `.kiro/steering/principles.md`: プロセス境界の明確化原則

---

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-10-13 | 1.0 | Claude Code | 初版作成（Sender/Receiver Concurrent Architecture、ADR-008欠陥解決） |
