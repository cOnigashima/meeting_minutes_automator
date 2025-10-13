# ADR-011: IPC Stdin/Stdout Mutex Separation

**Date**: 2025-10-13
**Status**: ❌ **Superseded** by ADR-013 (2025-10-14)
**Related**: ADR-008 (Rejected), ADR-009 (Rejected), ADR-012 (Audio Backpressure), ADR-013 (Approved)

---

## Context

ADR-009が提案した**Sender/Receiver Concurrent Architecture**は、以下の構造的欠陥（P0）を持つことが判明しました（本ADRは暫定対策として検討され、最終的にADR-013で統合されています）：

### 問題1: Mutex共有によるシリアライゼーション

**ADR-009の設計**:
```rust
pub struct PythonSidecarManager {
    child: Arc<Mutex<Child>>,
    // stdin/stdoutの直接アクセス不可
}

// Sender Task
let sender = Arc::clone(&python_sidecar);
let mut sidecar = sender.lock().await;  // ← Mutex取得
sidecar.send_message(...).await;        // ← .awaitでMutex保持

// Receiver Task
let receiver = Arc::clone(&python_sidecar);
let mut sidecar = receiver.lock().await; // ← Mutex取得待ち（Senderが保持中）
sidecar.receive_message().await;
```

**問題の本質**:
- `Arc<Mutex<PythonSidecarManager>>`を共有
- `send_message().await`中、Mutexを保持し続ける（tokio::Mutexの制限）
- Receiver Taskは`lock().await`で待機 → **並行実行が実質シリアライズ**
- ADR-008の「1フレーム送信 → 応答待ち → 次フレーム送信」構造的デッドロックが解消されていない

**影響範囲**:
- 🔴 **P0 Blocker**: Sender/Receiver並行化の目的が達成できない
- 🔴 **P0 Blocker**: 複数フレーム送信前にspeech_endを受信できない（Whisper特性）
- 🔴 **P0 Blocker**: 長時間発話（100秒超）でデッドロック再発

---

## Decision

**stdin/stdoutを独立したMutexに分離し、真の全二重通信を実現します。**

### 新構造体設計

```rust
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::Mutex;
use std::sync::Arc;

pub struct PythonSidecarManager {
    /// Stdin for sending JSON messages (独立したMutex)
    stdin: Arc<Mutex<ChildStdin>>,

    /// Stdout for receiving JSON messages (独立したMutex)
    stdout: Arc<Mutex<BufReader<ChildStdout>>>,

    /// Child process handle (監視のみ、IPC操作には使わない)
    child_handle: Arc<Mutex<Child>>,
}
```

### Send/Receive実装

```rust
impl PythonSidecarManager {
    /// Send JSON message to Python (stdinのみロック)
    pub async fn send_message(&self, msg: &serde_json::Value) -> Result<(), IpcError> {
        let json_line = serde_json::to_string(msg)? + "\n";

        let mut stdin = self.stdin.lock().await;  // ← stdin専用Mutex
        stdin.write_all(json_line.as_bytes()).await?;
        stdin.flush().await?;
        // ← Mutex即座に解放（.await後は自動的に解放）

        Ok(())
    }

    /// Receive JSON message from Python (stdoutのみロック)
    pub async fn receive_message(&self) -> Result<serde_json::Value, IpcError> {
        let mut stdout = self.stdout.lock().await; // ← stdout専用Mutex
        let mut line = String::new();

        let n = stdout.read_line(&mut line).await?;
        if n == 0 {
            return Err(IpcError::ProcessExited);
        }

        let msg = serde_json::from_str(&line)?;
        // ← Mutex即座に解放

        Ok(msg)
    }
}
```

### Sender/Receiver Tasks（並行実行）

```rust
// Sender Task: 連続フレーム送信（stdoutをブロックしない）
tokio::spawn({
    let sidecar = Arc::clone(&python_sidecar);
    async move {
        while let Some(frame) = frame_rx.recv().await {
            // stdinのみロック（stdoutは自由）
            sidecar.send_message(&serde_json::json!({
                "type": "audio_frame",
                "data": frame.data,
            })).await?;
        }
    }
});

// Receiver Task: 連続イベント受信（stdinをブロックしない）
tokio::spawn({
    let sidecar = Arc::clone(&python_sidecar);
    async move {
        loop {
            // stdoutのみロック（stdinは自由）
            let event = sidecar.receive_message().await?;

            match event["eventType"].as_str() {
                Some("speech_end") => { /* ... */ },
                Some("partial_text") => { /* ... */ },
                _ => {}
            }
        }
    }
});
```

---

## Consequences

### Positive

✅ **真の全二重通信実現**: Sender/Receiverが本当に並行実行される
✅ **Mutex競合解消**: send中でもreceiveが可能、receive中でもsendが可能
✅ **デッドロック根本解決**: 複数フレーム送信前にspeech_end受信可能（Whisper要件満足）
✅ **長時間発話対応**: 100秒超発話でも連続送信可能（STT-REQ-007.7準拠）
✅ **既存コードへの影響最小**: `send_message()`/`receive_message()`のシグネチャ変更なし

### Negative

⚠️ **Child process監視の複雑化**: stdin/stdoutを分離したため、プロセス終了検出を別実装
⚠️ **エラーハンドリング追加**: stdin書き込みエラーとstdout読み込みエラーを独立処理

### Trade-offs

- **Mutex粒度**: stdin/stdout分離により、Mutexスコープが最小化（send/receive時のみ）
- **メモリオーバーヘッド**: `Arc<Mutex<T>>`が2つに増えるが、サイズは無視できるレベル（数十バイト）

---

**Supersession Note (2025-10-14)**  
本ADRで定義したstdin/stdout分離方針はADR-013「Sidecar Full-Duplex IPC Final Design」に統合され、FacadeベースのAPI設計やバッファ契約と共に正式採択されました。詳細な実装および後続のP0修正はADR-013およびADR-013 P0 Bug Fixesを参照してください。

---

## Alternatives Considered

### Alternative 1: Message Queue Based IPC (Rejected)

```rust
// tokio::sync::mpsc channelでキュー化
let (send_tx, send_rx) = mpsc::channel(1000);
let (recv_tx, recv_rx) = mpsc::channel(1000);
```

**Rejection理由**:
- stdin/stdoutの本質的な全二重性を活かせない
- 追加レイヤー（キュー管理）でレイテンシ増加
- デッドロック問題は解決するが、オーバーエンジニアリング

---

### Alternative 2: Lock-Free with Crossbeam Channels (Rejected)

```rust
use crossbeam::channel::{unbounded, Sender, Receiver};
```

**Rejection理由**:
- `ChildStdin`/`ChildStdout`はasync I/O（tokio::io traits）
- crossbeamは同期channel → blocking操作が必要
- 非同期ランタイムの利点を放棄することになる

---

## Implementation Plan

### Phase 1: PythonSidecarManager構造体変更（30分）

**File**: `src-tauri/src/stt/python_sidecar.rs` (L25-45)

**変更内容**:
```rust
pub struct PythonSidecarManager {
    stdin: Arc<Mutex<ChildStdin>>,
    stdout: Arc<Mutex<BufReader<ChildStdout>>>,
    child_handle: Arc<Mutex<Child>>,
}

impl PythonSidecarManager {
    pub fn new(mut child: Child) -> Result<Self, IpcError> {
        let stdin = child.stdin.take()
            .ok_or(IpcError::StdinNotAvailable)?;
        let stdout = child.stdout.take()
            .ok_or(IpcError::StdoutNotAvailable)?;

        Ok(Self {
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: Arc::new(Mutex::new(BufReader::new(stdout))),
            child_handle: Arc::new(Mutex::new(child)),
        })
    }
}
```

---

### Phase 2: send_message()/receive_message()実装（45分）

**File**: `src-tauri/src/stt/python_sidecar.rs` (L70-110)

**変更内容**:
- `send_message()`: `self.stdin.lock().await`でstdinのみロック
- `receive_message()`: `self.stdout.lock().await`でstdoutのみロック
- エラーハンドリング: `BrokenPipe`, `UnexpectedEof`をIpcErrorに変換

---

### Phase 3: Sender/Receiver Tasks実装（1時間）

**File**: `src-tauri/src/stt/mod.rs` (Recording Session Task内)

**変更内容**:
- Sender Task: `frame_rx.recv().await` → `send_message().await` ループ
- Receiver Task: `receive_message().await` → イベントディスパッチループ
- Graceful Shutdown: `frame_rx.close()` → Sender終了 → `send({"type": "stop"})` → Receiver終了

---

### Phase 4: Child Process監視実装（30分）

**File**: `src-tauri/src/stt/python_sidecar.rs` (L140-160)

**追加メソッド**:
```rust
impl PythonSidecarManager {
    /// Monitor child process exit
    pub async fn wait_for_exit(&self) -> Result<ExitStatus, IpcError> {
        let mut child = self.child_handle.lock().await;
        child.wait().await.map_err(|e| IpcError::ProcessWaitError(e))
    }

    /// Check if process is still alive
    pub fn is_alive(&self) -> bool {
        self.child_handle.try_lock()
            .map(|mut c| c.try_wait().ok().flatten().is_none())
            .unwrap_or(false)
    }
}
```

---

### Phase 5: E2E Tests（1.5時間）

**File**: `src-tauri/tests/ipc_full_duplex_test.rs`

**Test Cases**:
1. `test_concurrent_send_receive()`: 100フレーム送信中に50イベント受信
2. `test_long_utterance_no_deadlock()`: 120秒発話でデッドロックなし
3. `test_stdin_error_independence()`: stdin書き込みエラー時もreceive継続
4. `test_stdout_error_independence()`: stdout読み込みエラー時もsend継続

---

## Success Criteria

### Functional Requirements

✅ **Concurrent Send/Receive**: 同時に100フレーム/秒送信 + 50イベント/秒受信可能
✅ **No Mutex Contention**: Sender/Receiver並行実行時のMutex待機時間 < 1ms
✅ **Long Utterance Support**: 120秒連続発話でデッドロックなし

### Non-Functional Requirements

✅ **Latency**: イベント受信レイテンシ < 50ms (ADR-009と同等)
✅ **Memory Overhead**: 構造体サイズ増加 < 128 bytes
✅ **Backward Compatibility**: 既存のsend/receive呼び出しコード変更不要

---

## Metrics and Monitoring

### Concurrency Metrics

```rust
// SttSessionMetrics拡張
pub struct SttSessionMetrics {
    pub mutex_contention_count: AtomicU64,       // Mutex競合回数
    pub stdin_lock_duration_us: AtomicU64,       // stdin lock保持時間
    pub stdout_lock_duration_us: AtomicU64,      // stdout lock保持時間
    pub concurrent_operations_count: AtomicU64,  // 並行send+receive回数
}
```

### Alert Conditions

🚨 **mutex_contention_count > 100/秒**: Mutex設計を再検証
🚨 **stdin_lock_duration_us > 10000** (10ms): 異常な長時間保持
🚨 **stdout_lock_duration_us > 50000** (50ms): 異常な読み込み遅延

---

## Rollback Strategy

### Rollback Trigger

以下いずれかが発生した場合、即座にロールバック:

1. **Deadlock再発**: 120秒発話でsend/receive停止
2. **Mutex Contention過多**: 競合回数 > 100/秒
3. **レイテンシ劣化**: イベント受信レイテンシ > 100ms

### Rollback Steps

1. **Feature Flag無効化**: `config.enable_separated_ipc_mutex = false`
2. **ADR-009実装に復帰**: `Arc<Mutex<PythonSidecarManager>>`共有版
3. **Metrics確認**: ロールバック後24時間監視

---

## Related Documents

- **ADR-008**: Dedicated Session Task (Rejected - 構造的デッドロック)
- **ADR-009**: Sender/Receiver Concurrent Architecture (Rejected - Mutex共有問題)
- **ADR-012**: Audio Callback Backpressure Redesign (blocking_send問題解決)
- **Design Section 7.9**: IPC Protocol Architecture
- **Task 7.3.2**: Sender/Receiver並行タスク実装（本ADR対応）

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
