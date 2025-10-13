# ADR-012: Audio Callback Backpressure Redesign

**Date**: 2025-10-13
**Status**: ✅ **Proposed** - Replaces ADR-009 (Part 2/2)
**Related**: ADR-008 (Rejected), ADR-009 (Rejected), ADR-011 (IPC Mutex Separation)

---

## Context

ADR-009が提案した**Audio Callback Blocking Backpressure**は、以下の構造的欠陥（P0）を持つことが判明しました：

### 問題2: blocking_send()によるCPALストリーム停止

**ADR-009の設計**:
```rust
// Audio Callback (CPALのリアルタイムコンテキスト)
move |data: &[f32], _: &cpal::InputCallbackInfo| {
    let audio_data = data.to_vec();

    // blocking_send: バッファ空きまで待機（最大2秒）
    match frame_tx.blocking_send(audio_data) {
        Ok(_) => { /* success */ },
        Err(_) => {
            // Channel閉じている場合のみエラー
            eprintln!("Frame channel closed");
        }
    }
}
```

**問題の本質**:

1. **blocking_send()の動作**:
   - バッファ満杯（200フレーム）時、**空きが出るまで無期限待機**
   - Python側がハング/クラッシュした場合、200フレーム（2秒）分の送信が詰まる
   - → オーディオコールバックが**最大2秒間ブロック**

2. **CPALのリアルタイム制約**:
   - OSオーディオバッファは通常**128ms** (macOS/Windows)
   - コールバックは**数十μs以内**に返す必要がある
   - 2秒のブロック → OSバッファオーバーラン → **ストリーム停止**

3. **ADR-010テストの盲点**:
   - "frame drop rate < 5%"テストは正常動作のみ検証
   - 異常ケース（Python hang/crash）未検証
   - 実運用で初めて発覚するP0バグ

**影響範囲**:
- 🔴 **P0 Blocker**: Python異常時にオーディオストリーム停止（ユーザーに録音停止と誤認される）
- 🔴 **P0 Blocker**: 復旧不可能（ストリーム再起動必要）
- 🟡 **P1 UX**: エラーメッセージなしでストリーム停止（ユーザー混乱）

---

## Decision

**Audio Callback内でのblocking操作を禁止し、以下の戦略を採用します：**

### Option A (推奨): try_send() + Large Ring Buffer + UI Notification

**アーキテクチャ**:
```rust
use tokio::sync::mpsc;
use std::sync::atomic::{AtomicBool, Ordering};

// 大容量バッファ（200 frames = 2秒 → 500 frames = 5秒）
let (frame_tx, frame_rx) = mpsc::channel::<AudioFrame>(500);

// ドロップ検出フラグ
let frame_drop_detected = Arc::new(AtomicBool::new(false));

// Audio Callback
let drop_flag = Arc::clone(&frame_drop_detected);
move |data: &[f32], _: &cpal::InputCallbackInfo| {
    let audio_data = data.to_vec();

    // Non-blocking try_send
    match frame_tx.try_send(audio_data) {
        Ok(_) => { /* success */ },
        Err(mpsc::error::TrySendError::Full(_)) => {
            // ドロップ発生（Python異常の兆候）
            drop_flag.store(true, Ordering::Relaxed);
            // ← ここでreturn（コールバックは即座に戻る）
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            // Channel閉じている（正常終了）
        }
    }
}

// UI Notification Task（別タスクで監視）
tokio::spawn({
    let drop_flag = Arc::clone(&frame_drop_detected);
    async move {
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if drop_flag.load(Ordering::Relaxed) {
                // UIにPython異常を通知
                app_handle.emit_all("python_stt_error", {
                    "error": "Python STT process not responding",
                    "action": "Please restart recording"
                }).ok();
                break;
            }
        }
    }
});
```

**Consequences**:
- ✅ **CPALストリーム保護**: コールバックは常に即座に戻る（ブロックなし）
- ✅ **異常検出**: バッファ満杯 = Python異常 → UI通知でユーザー対応可能
- ⚠️ **音声フレームドロップ**: バッファ満杯時は古いフレームを優先（新フレームをドロップ）
- ✅ **復旧戦略明確**: ユーザーが録音再起動 → 正常復帰

---

### Option B: send_timeout() + Immediate Recording Stop

**アーキテクチャ**:
```rust
use tokio::time::timeout;
use std::time::Duration;

// Audio Callback
move |data: &[f32], _: &cpal::InputCallbackInfo| {
    let audio_data = data.to_vec();

    // 50msタイムアウト付きsend
    match frame_tx.blocking_send_timeout(audio_data, Duration::from_millis(50)) {
        Ok(_) => { /* success */ },
        Err(mpsc::error::SendTimeoutError::Timeout(_)) => {
            // タイムアウト → 録音即座停止
            app_handle.emit_all("recording_stopped", {
                "reason": "Python STT timeout"
            }).ok();
            // ← ここでreturn（コールバックは即座に戻る）
        }
        Err(mpsc::error::SendTimeoutError::Closed(_)) => {
            // Channel閉じている（正常終了）
        }
    }
}
```

**Consequences**:
- ✅ **CPALストリーム保護**: 50ms以内に必ず戻る
- ✅ **音声フレームドロップなし**: タイムアウト時は録音停止（フレーム破損なし）
- ⚠️ **ユーザー体験**: 突然の録音停止（驚く可能性）
- ⚠️ **誤検出リスク**: 一時的な負荷でも録音停止

---

### Option C: Separate Thread + Lock-Free Ring Buffer

**アーキテクチャ**:
```rust
use ringbuf::{HeapRb, traits::*};
use std::sync::Arc;

// Lock-free ring buffer（1000 frames = 10秒）
let ring = HeapRb::<AudioFrame>::new(1000);
let (mut producer, mut consumer) = ring.split();

// Audio Callback (Lock-free push)
move |data: &[f32], _: &cpal::InputCallbackInfo| {
    let audio_frame = AudioFrame { data: data.to_vec() };

    match producer.try_push(audio_frame) {
        Ok(_) => { /* success */ },
        Err(_) => {
            // Ring buffer満杯（Python異常）
            // ← ここでreturn（コールバックは即座に戻る）
        }
    }
}

// Dedicated Thread: Ring Buffer → mpsc channel
std::thread::spawn(move || {
    loop {
        if let Some(frame) = consumer.try_pop() {
            // Blocking send OK（別スレッドなのでCPAL影響なし）
            frame_tx.blocking_send(frame).ok();
        } else {
            std::thread::sleep(Duration::from_micros(100));
        }
    }
});
```

**Consequences**:
- ✅ **完全Lock-free**: Audio Callbackは常に数μs以内に戻る
- ✅ **最大バッファ容量**: 10秒分の音声を保持可能
- ⚠️ **複雑度増加**: Ring buffer管理 + 専用スレッド
- ⚠️ **メモリオーバーヘッド**: Ring buffer + mpsc channel の二重バッファリング

---

## Comparison of Options

| Aspect                  | Option A (try_send)              | Option B (send_timeout)          | Option C (Ring Buffer)         |
| ----------------------- | -------------------------------- | -------------------------------- | ------------------------------ |
| **CPAL保護**                | ✅ 即座にreturn                      | ✅ 50ms以内にreturn                  | ✅ 数μs以内にreturn                |
| **音声フレームドロップ**          | ⚠️ バッファ満杯時ドロップ                   | ✅ ドロップなし（録音停止）                   | ⚠️ Ring buffer満杯時ドロップ         |
| **ユーザー体験**              | ✅ UI通知で対応可能                     | ⚠️ 突然の録音停止                       | ✅ UI通知で対応可能                   |
| **実装複雑度**               | 🟢 Low（mpsc channel + flag）      | 🟢 Low（mpsc channel + timeout）   | 🟡 Medium（ringbuf + thread）    |
| **メモリオーバーヘッド**          | 🟢 Low（500 frames）               | 🟢 Low（200 frames）               | 🟡 Medium（1000 frames + 200）   |
| **Python異常時の挙動**         | ドロップ → UI通知                      | タイムアウト → 録音停止                    | ドロップ → UI通知                    |
| **誤検出リスク**              | 🟢 Low（500フレーム = 5秒耐性）          | 🟡 Medium（50msタイムアウト）            | 🟢 Low（1000フレーム = 10秒耐性）      |
| **Recommendation**      | **✅ Recommended（バランス最適）**       | Fallback（ドロップ許容不可な場合）           | Over-engineering（必要なし）        |

---

## Recommended Decision: Option A

**理由**:

1. **実装シンプル**: mpsc channel + AtomicBool のみ
2. **適切なトレードオフ**: 音声フレームドロップ vs CPALストリーム保護
3. **ユーザー対応可能**: UI通知で録音再起動を促す
4. **誤検出耐性**: 500フレーム（5秒）バッファで一時的負荷に対応

---

## Implementation Plan (Option A)

### Phase 1: mpsc Channelバッファ拡大（15分）

**File**: `src-tauri/src/stt/mod.rs` (L45)

**変更内容**:
```rust
// OLD: 200 frames (2秒)
let (frame_tx, frame_rx) = mpsc::channel::<AudioFrame>(200);

// NEW: 500 frames (5秒)
let (frame_tx, frame_rx) = mpsc::channel::<AudioFrame>(500);
```

---

### Phase 2: try_send() + Drop Detection実装（30分）

**File**: `src-tauri/src/stt/mod.rs` (Audio Callback)

**変更内容**:
```rust
// Drop detection flag
let frame_drop_detected = Arc::new(AtomicBool::new(false));

// Audio Callback
let drop_flag = Arc::clone(&frame_drop_detected);
let data_callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
    let audio_frame = AudioFrame {
        data: data.to_vec(),
        timestamp: Instant::now(),
    };

    match frame_tx.try_send(audio_frame) {
        Ok(_) => { /* success */ },
        Err(mpsc::error::TrySendError::Full(_)) => {
            // ドロップ発生（Python異常の兆候）
            drop_flag.store(true, Ordering::Relaxed);
            // Metrics更新
            metrics.frames_dropped.fetch_add(1, Ordering::Relaxed);
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            // Channel閉じている（正常終了）
        }
    }
};
```

---

### Phase 3: UI Notification Task実装（45分）

**File**: `src-tauri/src/stt/mod.rs` (L120)

**追加Task**:
```rust
// UI Notification Task
tokio::spawn({
    let drop_flag = Arc::clone(&frame_drop_detected);
    let app_handle = app_handle.clone();
    async move {
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;

            if drop_flag.load(Ordering::Relaxed) {
                // UIにPython異常を通知
                app_handle.emit_all("stt_error", serde_json::json!({
                    "error": "Python STT process not responding",
                    "action": "Please restart recording",
                    "severity": "critical"
                })).ok();

                // Metrics更新
                metrics.python_hangs_detected.fetch_add(1, Ordering::Relaxed);
                break;
            }
        }
    }
});
```

---

### Phase 4: フロントエンドエラーハンドリング（30分）

**File**: `src/lib/stores/sttStore.ts` (L85)

**追加リスナー**:
```typescript
// Python STT Error Listener
listen<SttError>('stt_error', (event) => {
  const { error, action, severity } = event.payload;

  if (severity === 'critical') {
    // 録音を強制停止
    stopRecording();

    // ユーザーにエラー通知
    notifications.error(error, {
      description: action,
      duration: 10000,  // 10秒表示
      actions: [
        { label: 'Restart Recording', onClick: () => startRecording() }
      ]
    });
  }
});
```

---

### Phase 5: E2E Tests（1.5時間）

**File**: `src-tauri/tests/audio_callback_backpressure_test.rs`

**Test Cases**:

#### Test 1: Python Hang Detection
```rust
#[tokio::test]
async fn test_python_hang_detection() {
    // Setup: Python process を故意にsleep
    let python_sidecar = start_python_with_hang(Duration::from_secs(10));

    // Audio callback開始（500フレーム送信）
    let (tx, rx) = mpsc::channel(500);
    for i in 0..500 {
        tx.try_send(AudioFrame::dummy()).unwrap();
    }

    // 501フレーム目でFull
    let result = tx.try_send(AudioFrame::dummy());
    assert!(matches!(result, Err(TrySendError::Full(_))));

    // UI通知が発行されることを確認
    let event = wait_for_event("stt_error", Duration::from_secs(1)).await;
    assert_eq!(event.severity, "critical");
}
```

#### Test 2: Normal Operation No Drop
```rust
#[tokio::test]
async fn test_normal_operation_no_drop() {
    // Setup: 正常なPython process
    let python_sidecar = start_python();

    // 10000フレーム送信（20秒相当）
    let (tx, rx) = mpsc::channel(500);
    for i in 0..10000 {
        tx.try_send(AudioFrame::dummy()).unwrap();
        tokio::time::sleep(Duration::from_millis(2)).await; // 10msフレーム間隔
    }

    // ドロップなし
    let metrics = get_metrics();
    assert_eq!(metrics.frames_dropped, 0);
}
```

#### Test 3: Temporary Load No Drop
```rust
#[tokio::test]
async fn test_temporary_load_no_drop() {
    // Setup: Python processに一時的な負荷（3秒処理遅延）
    let python_sidecar = start_python_with_delay(Duration::from_secs(3));

    // 5秒間連続送信（500フレーム = 5秒バッファ）
    let (tx, rx) = mpsc::channel(500);
    for i in 0..500 {
        tx.try_send(AudioFrame::dummy()).unwrap();
    }

    // 3秒負荷でもドロップなし（5秒バッファ内）
    let metrics = get_metrics();
    assert_eq!(metrics.frames_dropped, 0);
}
```

---

## Success Criteria

### Functional Requirements

✅ **CPAL Protection**: Audio callbackが**常に10μs以内**に戻る（blocking操作なし）
✅ **Python Hang Detection**: バッファ満杯（500フレーム）時、**100ms以内**にUI通知
✅ **Normal Operation**: 正常動作時（Python応答正常）、**フレームドロップ率 < 0.01%**
✅ **Temporary Load Tolerance**: 3秒以内のPython遅延なら**ドロップなし**

### Non-Functional Requirements

✅ **Latency**: Audio callback遅延 < 10μs（ADR-009と同等）
✅ **Memory**: バッファ増加（200 → 500フレーム）= +300 frames × 1920 bytes = 576 KB
✅ **CPU**: UI Notification Task（100ms polling）= CPU使用率 < 0.1%

---

## Metrics and Monitoring

### Frame Drop Metrics

```rust
// SttSessionMetrics拡張
pub struct SttSessionMetrics {
    pub frames_dropped: AtomicU64,           // ドロップされたフレーム数
    pub python_hangs_detected: AtomicU64,    // Python hang検出回数
    pub callback_duration_us: AtomicU64,     // Audio callback処理時間（μs）
}
```

### Alert Conditions

🚨 **frames_dropped > 100**: Python異常（UI通知発行）
🚨 **python_hangs_detected > 1**: Python頻繁なhang（再起動推奨）
🚨 **callback_duration_us > 100**: Audio callback遅延異常（CPALストリーム停止リスク）

---

## Rollback Strategy

### Rollback Trigger

以下いずれかが発生した場合、即座にロールバック:

1. **CPALストリーム停止**: 正常動作でストリーム停止発生
2. **高頻度フレームドロップ**: 正常動作でframes_dropped > 1000/分
3. **UI通知誤発行**: Python正常でもUI通知発行

### Rollback Steps

1. **Feature Flag無効化**: `config.enable_try_send_backpressure = false`
2. **ADR-009実装に復帰**: `blocking_send()`版（ただし構造的欠陥あり）
3. **Option B検討**: `send_timeout()`実装を緊急開発

---

## Risk Analysis

### Risk 1: 音声フレームドロップによる文字起こし精度低下

**Likelihood**: 🟡 Medium（Python異常時のみ）
**Impact**: 🟡 Medium（ドロップ区間の音声が欠ける）

**Mitigation**:
- バッファサイズ500フレーム（5秒）で一時的負荷に耐性
- UI通知で即座にユーザー対応可能
- ドロップ時は録音再起動（音声破損よりマシ）

---

### Risk 2: UI通知の誤発行（False Positive）

**Likelihood**: 🟢 Low（500フレーム = 5秒耐性）
**Impact**: 🟢 Low（ユーザーが再起動するだけ）

**Mitigation**:
- 500フレームバッファで通常の負荷スパイクは吸収
- UI通知にドロップ率を表示（「5%以下なら無視」等）

---

### Risk 3: Memory Overflow（バッファ拡大による）

**Likelihood**: 🟢 Low（576 KB増加のみ）
**Impact**: 🟢 Low（現代PCでは無視できるレベル）

**Mitigation**:
- 最大メモリ使用量をモニタリング
- 異常増加時はバッファサイズを動的調整

---

## Alternatives Considered (Summary)

| Alternative                       | Status   | Reason                                          |
| --------------------------------- | -------- | ----------------------------------------------- |
| Option A: try_send() + UI Notify  | ✅ Adopted | バランス最適、実装シンプル                                   |
| Option B: send_timeout()          | ⏸️ Backup | ドロップ許容不可な場合のFallback                           |
| Option C: Ring Buffer + Thread    | ❌ Rejected | Over-engineering、複雑度増加に見合う利点なし                  |
| Keep blocking_send() (ADR-009)    | ❌ Rejected | Python異常時にCPALストリーム停止（P0 Blocker）               |

---

## Related Documents

- **ADR-008**: Dedicated Session Task (Rejected - 構造的デッドロック)
- **ADR-009**: Sender/Receiver Concurrent Architecture (Rejected - blocking_send問題)
- **ADR-011**: IPC Stdin/Stdout Mutex Separation (Mutex共有問題解決)
- **Design Section 7.9**: IPC Protocol Architecture
- **Task 7.3.3**: Audio Callback Blocking Backpressure実装（本ADR対応）

---

## Approval

- [ ] Tech Lead Review
- [ ] Implementation Complete
- [ ] E2E Tests Pass
- [ ] Production Deployment

---

**Document Version**: v1.0
**Created**: 2025-10-13
**Status**: ✅ Proposed
