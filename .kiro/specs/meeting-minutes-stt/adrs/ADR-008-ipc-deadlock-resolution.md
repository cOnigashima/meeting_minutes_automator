# ADR-008: IPC Deadlock解決アーキテクチャ (Dedicated Session Task)

## Status
❌ **Rejected (2025-10-13)** - Superseded by ADR-009

**Rejection Reason**: 外部レビュー（2025-10-13）により、以下の3つの致命的欠陥が発見されました：

1. **構造的デッドロック（P0）**: 1フレーム送信→speech_end待ち→次フレーム送信の順序により、Whisperが複数フレームなしでspeech_endを出せず永久デッドロック（既存実装と同じ問題）
2. **Python偽no_speech検出（P1）**: イベント発行の有無だけで判定するため、発話継続中でもイベント間にno_speechを誤送信
3. **Backpressure Frameドロップ**: 10msフレームの無条件ドロップが音声ストリーム破損と文字起こし精度低下を招く

**解決策**: ADR-009（Sender/Receiver Concurrent Architecture）で根本解決

## Context

### 問題の発見
Task 7.1.6（Event Stream Protocol）実装において、外部レビューにより以下の3つの致命的なデッドロックが発見されました：

#### 1. Mutex Await問題
```rust
// 現在の実装（問題あり）
audio_callback {
    tokio::spawn(async move {
        let mut sidecar = python_sidecar.lock().await;  // ACQUIRE
        sidecar.send_message(request).await;
        loop {
            sidecar.receive_message().await;  // BLOCKS, STILL HOLDING MUTEX
        }
    });
}
```

**問題点**:
- `MutexGuard` が `.await` 中も保持され続ける
- 次のオーディオチャンク（100ms後）がmutex取得待ちでブロック
- 長い発話（>5秒）で次のチャンクが送信不可 → Pythonが `speech_end` を検出できない → 永久デッドロック

#### 2. タイムアウトによる結果損失
```rust
// 暫定修正（問題あり）
match tokio::time::timeout(
    Duration::from_secs(5),
    sidecar.receive_message()
).await {
    Err(_) => break,  // Timeout!
}
```

**問題点**:
- Whisperの長時間処理（>5秒）でタイムアウト発動
- Rustタスクが終了し、`final_text` が未読のままパイプに残る
- 次のリクエストが古い `final_text` を読む → `requestId` 不一致

#### 3. キュー汚染
- 未読イベントが次のリクエストに混入
- クロスリクエスト汚染によりイベント処理が破綻

### 根本原因
**Per-chunk Request/Response モデルの限界**:
- 各オーディオチャンク（100ms間隔）が新しいタスクを起動
- 各タスクが独立して送信→受信ループを実行
- 受信ループ中にmutexを保持 → 並行性が完全に失われる

## Decision

**採用アーキテクチャ**: Alternative 2 - Dedicated Session Task

**外部レビュー検証結果（2025-10-13）**:
- 外部レビュアーの「代替案3: セッション単位の非同期タスク」と本ADRのAlternative 2は実質的に同一アプローチ
- Alternative 1 (Global IPC Reader)に対する批判的指摘は全て正当（Bootstrap sequencing問題、Shared receiver競合、Error policy未定義）
- Alternative 2に対する指摘は誤解（"ブロードキャスト受信の共有"は実際には発生しない）
- **最終判定**: Alternative 2を維持すべき（最もシンプルで堅牢）

### 3つのアーキテクチャ比較

**重要**: この比較は実装前の設計検討です。最終的にAlternative 2を採用しました。

#### Alternative 1: Global IPC Reader Task + Broadcast Channel
```rust
// グローバルリーダータスク（1個のみ）
tokio::spawn(async move {
    loop {
        let event = { /* receive with mutex */ };
        event_tx.send(event).unwrap();  // Broadcast to all
    }
});

// 各オーディオコールバック
audio_callback {
    let mut event_rx = event_tx.subscribe();  // Own subscription
    tokio::spawn(async move {
        loop {
            let event = event_rx.recv().await;
            if event.request_id == request_id { /* process */ }
        }
    });
}
```

**メリット**:
- ✅ Mutexを `.await` 跨ぎで保持しない
- ✅ タイムアウト不要

**デメリット**:
- ❌ **Bootstrap sequencing問題**: `python_sidecar` が `None` の状態でリーダー起動できない
- ❌ **Shared receiver競合**: 計画ミスにより全タスクが1つの `Receiver` を共有するとbroadcastの意味が失われる
- ❌ **Error policy未定義**: JSON parse失敗時に無効ペイロードをbroadcast
- ❌ **複雑性**: 新しい並行プリミティブ、100行以上の変更

**結論**: リスク高すぎる。却下。

---

#### Alternative 2: Dedicated Session Task (採用)
```rust
pub async fn start_recording() {
    let (frame_tx, mut frame_rx) = mpsc::channel(100);
    let (event_tx, _) = broadcast::channel(1000);

    // Single session task for entire recording
    tokio::spawn(async move {
        while let Some(frame) = frame_rx.recv().await {
            // Send frame to Python
            {
                let mut sidecar = python_sidecar.lock().await;
                sidecar.send_message(frame).await;
            } // Mutex dropped immediately

            // Receive events until terminal
            loop {
                let event = {
                    let mut sidecar = python_sidecar.lock().await;
                    sidecar.receive_message().await
                };

                match event {
                    Ok(msg) => {
                        event_tx.send(msg.clone());

                        // Check terminal events
                        if is_terminal(&msg) { break; }
                    }
                    Err(e) => {
                        eprintln!("IPC error: {:?}", e);
                        break;
                    }
                }
            }
        }
    });

    // Audio callback just pushes frames (non-blocking)
    device.start_with_callback(move |frame| {
        let _ = frame_tx.try_send(frame);  // Non-blocking
    });
}
```

**メリット**:
- ✅ **シンプル**: 1セッション1タスクのみ
- ✅ **Request ID不要**: セッション単位で管理
- ✅ **Backpressure自然**: mpsc bounded channel（100フレーム = 10秒バッファ）
- ✅ **初期化順序明確**: `start_recording` 時にタスク起動
- ✅ **エラー処理明確**: セッションタスクが終了 → 録音終了
- ✅ **既存テストへの影響最小**

**デメリット**:
- 中程度の変更量（50-80行）

**結論**: **採用**。最適なバランス。

---

#### Alternative 3: Split Mutex (却下)
```rust
pub struct PythonSidecarManager {
    send_mutex: Arc<tokio::Mutex<ChildStdin>>,
    recv_mutex: Arc<tokio::Mutex<ChildStdout>>,
}
```

**メリット**:
- ✅ 最小限の変更

**デメリット**:
- ❌ **デッドロック未解決**: 依然として各タスクが `recv_mutex.lock().await` で待機
- ❌ 長い発話時のmutex競合は未解決

**結論**: 問題を解決しない。却下。

---

## Consequences

### Positive
1. **デッドロック完全解消**:
   - Mutexを `.await` 跨ぎで保持しない
   - タイムアウト不要（イベントは必ず到着）
   - キュー汚染なし（セッション単位で管理）

2. **アーキテクチャの単純化**:
   - Request/Responseパターン → Session/Eventパターン
   - Per-chunk並行性 → Per-session並行性
   - グローバル状態不要

3. **自然なBackpressure**:
   - mpsc bounded channel（100フレーム = 10秒バッファ）
   - オーディオコールバックがブロックせず、古いフレームをdrop

### Negative
1. **中程度のリファクタリング**:
   - `start_recording` 完全書き換え
   - 50-80行の変更
   - 1-2日の作業時間

2. **Per-requestモデルからの脱却**:
   - 既存の「各チャンクが独立したリクエスト」という概念を放棄
   - セッション全体を1つのライフサイクルで管理

## Implementation

### Phase 1: Session Task Infrastructure (2-3時間)

#### 1.1 Channel Setup
```rust
// start_recording()
let (frame_tx, frame_rx) = mpsc::channel::<Vec<u8>>(100);  // 10s buffer
let (event_tx, _) = broadcast::channel::<serde_json::Value>(1000);
```

#### 1.2 Session Task
```rust
fn spawn_recording_session_task(
    python_sidecar: Arc<tokio::Mutex<PythonSidecarManager>>,
    websocket_server: Arc<tokio::Mutex<WebSocketServer>>,
    mut frame_rx: mpsc::Receiver<Vec<u8>>,
    event_tx: broadcast::Sender<serde_json::Value>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(audio_data) = frame_rx.recv().await {
            // Send frame
            {
                let mut sidecar = python_sidecar.lock().await;
                let request = ProtocolMessage::Request {
                    id: format!("audio-{}", SystemTime::now().as_millis()),
                    version: PROTOCOL_VERSION.to_string(),
                    method: "process_audio_stream".to_string(),
                    params: serde_json::json!({ "audio_data": audio_data }),
                };
                sidecar.send_message(serde_json::to_value(&request).unwrap()).await;
            }

            // Receive events until terminal
            loop {
                let event_result = {
                    let mut sidecar = python_sidecar.lock().await;
                    sidecar.receive_message().await
                };

                match event_result {
                    Ok(event) => {
                        // Broadcast to subscribers (WebSocket, UI, etc.)
                        let _ = event_tx.send(event.clone());

                        // Parse and check terminal
                        if let Ok(msg) = serde_json::from_value::<ProtocolMessage>(event) {
                            match msg {
                                ProtocolMessage::Event { event_type, data, .. } => {
                                    // Forward to WebSocket
                                    if event_type == "final_text" {
                                        let ws_msg = WebSocketMessage::Transcription {
                                            // ... construct from data ...
                                        };
                                        let ws_server = websocket_server.lock().await;
                                        let _ = ws_server.broadcast(ws_msg).await;
                                    }

                                    // Terminal events
                                    if event_type == "speech_end" || event_type == "no_speech" {
                                        break;
                                    }
                                }
                                ProtocolMessage::Error { .. } => {
                                    eprintln!("Python error: {:?}", msg);
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("IPC receive error: {:?}", e);
                        break;
                    }
                }
            }
        }

        eprintln!("[Session Task] Recording session ended");
    })
}
```

#### 1.3 Audio Callback
```rust
device.start_with_callback(move |audio_data| {
    // Non-blocking send (drops frame if channel full)
    if let Err(e) = frame_tx.try_send(audio_data) {
        eprintln!("[Audio Callback] Frame dropped: {:?}", e);
        // This is OK - backpressure signal
    }
});
```

### Phase 2: Error Handling (1時間)

#### 2.1 Exponential Backoff
```rust
let mut error_count = 0;

match sidecar.receive_message().await {
    Err(e) => {
        error_count += 1;
        let backoff = Duration::from_millis(100 * 2u64.pow(error_count.min(5)));
        eprintln!("IPC error #{}: {:?}, waiting {:?}", error_count, e, backoff);
        tokio::time::sleep(backoff).await;

        if error_count > 10 {
            eprintln!("Too many errors, terminating session");
            break;
        }
    }
}
```

#### 2.2 JSON Parse Error Handling (外部レビュー対応)
```rust
// Session Task内でイベント受信時
match event_result {
    Ok(event) => {
        // Parse IPC message with error handling
        match serde_json::from_value::<ProtocolMessage>(event.clone()) {
            Ok(msg) => {
                // Valid message - broadcast to subscribers
                let _ = event_tx.send(event.clone());

                // Process message
                match msg {
                    ProtocolMessage::Event { event_type, .. } => {
                        if event_type == "speech_end" || event_type == "no_speech" {
                            break;
                        }
                    }
                    ProtocolMessage::Error { .. } => break,
                    _ => {}
                }
            }
            Err(e) => {
                eprintln!("[Session Task] Invalid JSON from Python: {:?}", e);
                // DO NOT broadcast corrupted event
                // Continue to next event (don't break session)
                error_count += 1;
                if error_count > 10 {
                    eprintln!("[Session Task] Too many parse errors, terminating");
                    break;
                }
            }
        }
    }
    Err(e) => {
        // IPC receive error
        error_count += 1;
        let backoff = Duration::from_millis(100 * 2u64.pow(error_count.min(5)));
        tokio::time::sleep(backoff).await;

        if error_count > 10 {
            break;
        }
    }
}
```

**重要**: JSONパースエラー時は破損イベントをbroadcastせず、次のイベントを待つ。10回連続エラーでセッション終了。

#### 2.3 Python Process Health Monitoring (外部レビュー対応)
```rust
// start_recording()内で監視タスク起動
fn spawn_python_health_monitor(
    python_sidecar: Arc<tokio::Mutex<PythonSidecarManager>>,
    app: AppHandle,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut check_count = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;

            let is_alive = {
                let sidecar = python_sidecar.lock().await;
                sidecar.is_process_alive()  // Check child process status
            };

            if !is_alive {
                eprintln!("[Health Monitor] Python process died, attempting restart");

                // Notify UI
                let _ = app.emit(
                    "python-process-error",
                    serde_json::json!({
                        "type": "process_died",
                        "message": "Python音声処理プロセスが異常終了しました",
                        "recoverable": true,
                    }),
                );

                // Attempt restart (max 3 retries)
                check_count += 1;
                if check_count > 3 {
                    eprintln!("[Health Monitor] Max retries exceeded, giving up");
                    break;
                }

                // Restart logic (to be implemented in PythonSidecarManager)
                let mut sidecar = python_sidecar.lock().await;
                if let Err(e) = sidecar.restart().await {
                    eprintln!("[Health Monitor] Restart failed: {:?}", e);
                } else {
                    eprintln!("[Health Monitor] Restart succeeded");
                    check_count = 0;  // Reset counter on success
                }
            } else {
                check_count = 0;  // Reset on healthy check
            }
        }
    })
}
```

**実装メモ**: `PythonSidecarManager::is_process_alive()` と `restart()` メソッドを追加実装する必要あり。

#### 2.4 Session Metrics Collection (外部レビュー対応)
```rust
struct SessionMetrics {
    frames_sent: AtomicU64,
    frames_dropped: AtomicU64,
    events_received: AtomicU64,
    parse_errors: AtomicU64,
    ipc_errors: AtomicU64,
    start_time: Instant,
}

impl SessionMetrics {
    fn report(&self) {
        let duration = self.start_time.elapsed();
        eprintln!(
            "[Session Metrics] Duration: {:?}, Frames: sent={}, dropped={}, Events: received={}, Errors: parse={}, ipc={}",
            duration,
            self.frames_sent.load(Ordering::Relaxed),
            self.frames_dropped.load(Ordering::Relaxed),
            self.events_received.load(Ordering::Relaxed),
            self.parse_errors.load(Ordering::Relaxed),
            self.ipc_errors.load(Ordering::Relaxed),
        );
    }
}
```

**用途**: デバッグ・運用監視。セッション終了時に自動レポート。

#### 2.5 Graceful Shutdown
```rust
// stop_recording()
frame_tx.close();  // Signal session task to finish
health_monitor_handle.abort();  // Stop health monitor
session_handle.await;  // Wait for clean shutdown

// Log final metrics
metrics.report();
```

### Phase 3: Testing (2-3時間)

#### 3.1 Unit Tests
```rust
#[tokio::test]
async fn test_session_task_processes_frames() {
    let (frame_tx, frame_rx) = mpsc::channel(10);
    let (event_tx, mut event_rx) = broadcast::channel(100);

    // Spawn session task with mock sidecar
    let handle = spawn_recording_session_task(/* ... */);

    // Send 10 frames
    for i in 0..10 {
        frame_tx.send(vec![i; 320]).await.unwrap();
    }

    // Verify 10 events received
    let mut count = 0;
    while let Ok(_) = tokio::time::timeout(Duration::from_millis(100), event_rx.recv()).await {
        count += 1;
    }
    assert_eq!(count, 10);
}
```

#### 3.2 Integration Tests
```rust
#[tokio::test]
async fn test_long_utterance_no_deadlock() {
    // Send 100 frames (10 seconds of audio)
    // Verify all events received without timeout
}

#[tokio::test]
async fn test_backpressure_drops_frames() {
    // Fill mpsc channel to capacity
    // Verify try_send returns Err
    // Verify recording continues after backpressure clears
}
```

#### 3.3 E2E Tests (必須)
```rust
// e2e_test.rs
#[tokio::test]
async fn test_real_tauri_app_with_fake_python() {
    // 1. Boot Tauri app
    let app = tauri::test::mock_app();

    // 2. Start fake Python sidecar
    let fake_python = FakePythonSidecar::new();

    // 3. Send 100 audio chunks
    for i in 0..100 {
        app.emit("audio_chunk", generate_chunk(i)).await;
    }

    // 4. Verify all events received in order
    let events = fake_python.get_all_events();
    assert_eq!(events.len(), 100);

    // 5. Verify no events lost or duplicated
    for (i, event) in events.iter().enumerate() {
        assert_eq!(event["frame_id"], i);
    }
}
```

### Phase 4: Documentation (30分)

#### 4.1 Code Comments
```rust
/// Recording Session Task Architecture (ADR-008)
///
/// This system uses a dedicated session task per recording to avoid deadlocks.
///
/// Problem (before):
/// - Each audio chunk held python_sidecar mutex while waiting for events
/// - Long Whisper processing (>5s) blocked other chunks from sending
/// - Timeout caused legitimate results to be lost
///
/// Solution (after):
/// - Single session task processes all frames for one recording
/// - Mutex held ONLY during send/receive operations
/// - Events broadcast to all subscribers (WebSocket, UI, etc.)
/// - No timeout needed (events always arrive eventually)
```

#### 4.2 Requirement Traceability
```markdown
| STT-REQ-007.7 | IPC長時間処理対応 | ADR-008 | ✅ 完了 | commands.rs:spawn_recording_session_task | tests/session_task_test.rs |
```

### Implementation Files
- `src-tauri/src/commands.rs`: Session task implementation
- `src-tauri/src/state.rs`: Session handle storage (optional)
- `src-tauri/tests/session_task_test.rs`: Unit tests
- `src-tauri/tests/e2e_test.rs`: End-to-end tests

## Success Criteria

### 必須
- ✅ 全既存テスト合格（Rust 26 + Python 143）
- ✅ 長時間処理（>5秒）でタイムアウトなし
- ✅ 100並行チャンクでデッドロックなし（E2Eテスト）
- ✅ 無音チャンクで `no_speech` 正常受信
- ✅ Backpressure時にフレームをdrop（ログ出力）

### 望ましい
- ✅ 2分録音が数秒で処理完了
- ✅ Pythonクラッシュでセッションタスクが正常終了
- ✅ メモリリークなし（Valgrind検証）

### 計測メトリクス
```rust
struct SessionMetrics {
    frames_sent: AtomicU64,
    frames_dropped: AtomicU64,
    events_received: AtomicU64,
    avg_latency_ms: AtomicU64,
}

// 10秒ごとにログ出力
tokio::spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;
        eprintln!(
            "[Session Metrics] Sent: {}, Dropped: {}, Received: {}, Latency: {}ms",
            metrics.frames_sent.load(Ordering::Relaxed),
            metrics.frames_dropped.load(Ordering::Relaxed),
            metrics.events_received.load(Ordering::Relaxed),
            metrics.avg_latency_ms.load(Ordering::Relaxed)
        );
    }
});
```

## Rollback Strategy

### Phase 1: Feature Flag
```rust
const USE_SESSION_TASK: bool = false;  // Default: OFF

pub async fn start_recording() {
    if USE_SESSION_TASK {
        start_recording_with_session_task().await
    } else {
        start_recording_legacy().await  // Current implementation
    }
}
```

### Phase 2: Gradual Rollout
1. Week 1: Internal testing only (`USE_SESSION_TASK = true` for developers)
2. Week 2: Beta users (10%)
3. Week 3: Full rollout (100%)

### Rollback Trigger
- デッドロック発生率 > 1%
- フレームdrop率 > 5%
- ユーザークレーム > 3件

### Rollback Procedure
1. `USE_SESSION_TASK = false` に戻す
2. Hot-fix release（1時間以内）
3. Post-mortem作成

## References
- STT-REQ-007: Event Stream Protocol (IPC Protocol Extension)
- Task 7.1.6: Event Stream Protocol実装
- 外部レビュー指摘（2025-10-13）: Mutex await問題、タイムアウト結果損失、Shared receiver競合
- `.kiro/steering/principles.md`: プロセス境界の明確化原則

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-10-13 | 1.0 | Claude Code | 初版作成（3アーキテクチャ比較、Alternative 2採用） |
| 2025-10-13 | 1.1 | Claude Code | **外部レビュー対応**: JSONパースエラー処理（Phase 2.2）、Pythonプロセス監視（Phase 2.3）、セッションメトリクス（Phase 2.4）追加。Decision セクション明確化（レビュアー代替案3と同一であることを明記）。 |
