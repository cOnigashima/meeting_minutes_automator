# 意思決定記録: IPC Architecture Final Decision

**決定日**: 2025-10-14
**決定者**: Technical Review + Critical Analysis
**対象仕様**: meeting-minutes-stt (MVP1)
**関連ADR**: ADR-013

---

## TL;DR

**決定**: ADR-013（Sidecar Full-Duplex IPC Final Design）を採用し、ADR-011/012を正式に置き換える。

**主要変更点**:
1. ✅ Sidecar APIを`AudioSink`/`EventStream` facadeに刷新（Mutex隠蔽）
2. ✅ Framing ProtocolをLine-Delimited JSONに明示化（read_exact() deadlock回避）
3. ✅ Buffer戦略を5秒 + 即座停止に確定（120秒 + 自動Pause案を不採用）

**実装開始**: 本日より（推定工数: 3日）

---

## 背景

### ADR-011/012の課題

ADR-011/012は正しい方向性を示していたが、以下3点の不明確さが残っていた：

1. **API Ambiguity**: `Arc<Mutex<ChildStdin/Stdout>>`の具体的な使い方が不明
2. **Framing Unspecified**: Rust → Pythonの音声データ送信プロトコルが未定義
3. **Buffer Policy Contradiction**: 5秒バッファ vs 2秒ブロックの矛盾

### 外部レビュー結果

提案された「Ring Buffer + STDIO Full-Duplex」アーキテクチャに対し、以下の致命的欠陥が指摘された：

#### Critical Flaw #1: 120秒バッファ + 自動Pause戦略の矛盾

**問題**:
- 120秒バッファ（3.7 MB） + 85%で自動Pause → バッファ保持が保証されない
- CPAL `pause()`はプラットフォーム依存（macOS: 保持、Windows: 不定、Linux: 消失）
- Fallback `stop() + start()`でバッファ消失 → フレームロス

**結論**: 自動Pauseは不採用

#### Critical Flaw #2: read_exact(320) Deadlock Risk

**問題**:
- Pythonの`sys.stdin.buffer.read_exact(320)`はブロッキング
- Rust側が320 bytes未満送信時、永久待機 → Deadlock
- CPAL callbackの不規則性により、固定長保証は不可能

**結論**: Line-Delimited JSONに変更（行境界で必ず区切れる）

---

## 採用決定事項

### 1. Sidecar Facade API

#### 決定内容

```rust
pub struct Sidecar {
    pub sink: AudioSink,       // mpsc::Sender<Bytes> facade
    pub events: EventStream,   // broadcast::Receiver<Event> facade
    ctrl: Control,             // Internal management
}

// アプリ側はチャネルのみに触れる（Mutex不要）
impl AudioSink {
    pub async fn send_frame(&self, frame: bytes::Bytes) -> Result<()> {
        self.tx.send(frame).await
    }
}

impl EventStream {
    pub async fn recv(&mut self) -> anyhow::Result<Event> {
        self.rx.recv().await  // Auto lag handling
    }
}
```

#### 理由

- ✅ ChildStdin/ChildStdoutを完全隠蔽（アプリがMutexを意識しない）
- ✅ 内部で writer/reader タスクが独立動作（フルデュープレックス保証）
- ✅ 既存コードへの影響最小（新API追加 + 段階移行可能）

#### 実装優先度

🔴 P0 Critical - 1日

---

### 2. Line-Delimited JSON Framing

#### 決定内容

**Rust → Python**:
```rust
let msg = serde_json::json!({
    "type": "audio_frame",
    "data": base64::encode(pcm_bytes),
    "sample_rate": 16000,
});
let line = serde_json::to_string(&msg)? + "\n";
sink.send_frame(line.into()).await?;
```

**Python → Rust**:
```python
for line in sys.stdin:  # ← 行単位で必ず区切れる
    msg = json.loads(line)
    if msg["type"] == "audio_frame":
        frame = base64.b64decode(msg["data"])
        ingest_queue.put(frame, timeout=5.0)
```

#### Overhead分析

| Metric       | Value       | Impact       |
| ------------ | ----------- | ------------ |
| Raw PCM      | 320 B       | Baseline     |
| Base64       | ~427 B      | +33%         |
| JSON wrapper | ~450 B      | +40% total   |
| **帯域幅**      | 45 KB/s     | Negligible   |

#### 理由

- ✅ read_exact() Deadlock完全回避（行境界保証）
- ✅ 既存stdout実装と対称性（JSON per line統一）
- ✅ 実装工数最小（既存パーサー再利用）
- ✅ デバッグ容易（Text format）

#### 実装優先度

🔴 P0 Critical - 0.5日

---

### 3. Buffer Strategy: 5s + Immediate Stop

#### 決定内容

| 項目           | 値                | 理由                  |
| ------------ | ---------------- | ------------------- |
| **容量**       | 5秒 (160 KB)      | 十分な余裕 + メモリ効率       |
| **Overflow時** | 即座停止 + UI通知      | 決定的エラー検出（ADR-012準拠） |
| **自動Pause**  | なし               | バッファ消失リスク回避       |

#### 動作仕様

```rust
match ring_buffer.occupancy() {
    0.0..=0.5  => Level::Normal,
    0.5..=0.7  => Level::Warn,    // UI: "処理遅延"
    0.7..=1.0  => Level::Critical, // UI: "まもなく停止 (Xs)"
    1.0..      => {
        // 5秒到達 = Python異常
        stop_recording();
        emit_error("stt_error", {
            "error": "Python STT timeout (5 seconds)",
            "action": "Please restart recording",
            "severity": "critical"
        });
    }
}
```

#### 理由

- ✅ 5秒 = 通常のPython遅延（<1秒）に対し十分な余裕
- ✅ Overflow = 明確なシステム異常 → 即座通知でUX改善
- ✅ 自動Pause廃止 → バッファ保持問題・UX混乱を回避
- ✅ メモリ効率: 160 KB（vs 120秒案の3.7 MB）

#### 実装優先度

🔴 P0 Critical - 0.5日

---

### 4. Python Execution Model

#### 決定内容

```python
# Thread 1: stdin Reader (専用、GIL不要)
def stdin_reader():
    for line in sys.stdin:
        msg = json.loads(line)
        if msg["type"] == "audio_frame":
            frame = base64.b64decode(msg["data"])
            ingest_queue.put(frame, timeout=5.0)

# Thread 2: VAD/Aggregator
def vad_aggregator():
    # VAD判定 + 800ms単位でSTT送信
    # no_speech判定（VAD状態ベース）

# Thread 3: STT Worker
def stt_worker():
    # Whisper inference (GIL released)
    result = whisper.transcribe(batch)
    sys.stdout.write(json.dumps(result) + "\n")
    sys.stdout.flush()
```

#### Queue仕様

| Queue         | maxsize | Duration | Timeout |
| ------------- | ------- | -------- | ------- |
| ingest_queue  | 500     | 5秒分      | 5.0s    |
| stt_queue     | 100     | ~10秒分    | N/A     |

#### no_speech判定（修正版）

```python
# WRONG (ADR-008/009の誤り)
if not speech_detected:
    emit_no_speech()  # ← イベント未出力 = 無音と誤判定

# CORRECT (ADR-013)
if not speech_detected:
    if not pipeline.is_in_speech() and not pipeline.has_buffered_speech():
        # 物理的に無音
        emit_no_speech()
    else:
        # 発話継続中（イベント未出力だが音声あり）
        pass
```

#### 理由

- ✅ Reader Thread独立 → STT遅延の影響なし
- ✅ VAD/STT分離 → 並行処理でスループット向上
- ✅ Bounded Queue → Backpressure自動伝播（Rust 5秒バッファと同期）
- ✅ VAD状態ベース判定 → 偽no_speech完全防止

#### 実装優先度

🟡 P1 High - 1日

---

## 不採用決定事項

### ❌ 120秒バッファ戦略

**理由**:
- メモリ過大（3.7 MB vs 160 KB）
- UX不明確（異常検出まで最大120秒待機）
- 自動Pauseのバッファ消失リスク

---

### ❌ 自動Pause/Resume

**理由**:
- CPAL pause()のバッファ保持非保証（プラットフォーム依存）
- stop()+start() fallbackでバッファ消失 → フレームロス
- UX混乱（「停止したのに古い音声が流れる」）

---

### ❌ read_exact(320) 固定長フレーミング

**理由**:
- P0 Blocker: Rust側が320 bytes未満送信時にDeadlock
- CPAL callbackの不規則性（10ms保証なし）
- Ring Buffer pop側のチャンク化も不定

---

### ❌ Socket-Based Duplex Service

**理由**:
- 実装工数増（Protocol framing + Reconnection実装）
- Line-Delimited JSON STDIOで十分
- Kernel backpressureの利点 < 実装コスト

**位置づけ**: Plan-B（Sidecar分割が技術的に困難な場合のFallback）

---

### ❌ gRPC/WebRTC

**理由**:
- Over-engineering（音声ストリーム用途に過剰）
- Latency増加（HTTP/2 overhead）
- 実装工数大（5-7日）

---

## 実装ロードマップ

### Phase 1: Sidecar分離API（1日）

**担当**: Rust開発者
**期限**: Day 1

**タスク**:
- [ ] `Sidecar`/`AudioSink`/`EventStream` 構造体実装
- [ ] `spawn_stdio_writer`/`spawn_stdio_reader` 内部タスク実装
- [ ] Line-Delimited JSON framing（Rust → Python）
- [ ] 既存`PythonSidecarManager` 非推奨化（deprecation警告）

**成功基準**:
- ✅ 送信継続中でも受信が並行動作（ユニットテスト合格）
- ✅ 共有Mutexゼロ（所有権分離確認）

**ファイル**:
- `src-tauri/src/stt/sidecar.rs`（新規）
- `src-tauri/src/stt/python_sidecar.rs`（deprecated wrapper）

---

### Phase 2: Ring Buffer導入（0.5日）

**担当**: Rust開発者
**期限**: Day 1.5

**タスク**:
- [ ] SPSC Ring Buffer実装（5秒容量 = 160 KB）
- [ ] CPAL callback → ring.push()のみ
- [ ] Occupancy監視 + UI通知（Warn/Critical/Overflow）
- [ ] Overflow時の即座停止 + error emit

**成功基準**:
- ✅ Callback処理時間 <10μs（ベンチマーク確認）
- ✅ 5秒Python停止 → 5秒後に録音停止 + UI通知（E2Eテスト合格）

**ファイル**:
- `src-tauri/src/stt/ring_buffer.rs`（新規）
- `src-tauri/src/stt/mod.rs`（Audio callback変更）

---

### Phase 3: Python実行モデル（1日）

**担当**: Python開発者
**期限**: Day 2.5

**タスク**:
- [ ] stdin Reader Thread（Line-based JSON）
- [ ] VAD/Aggregator Thread（`is_in_speech()`/`has_buffered_speech()`）
- [ ] STT Worker Thread（Whisper C++ GIL release）
- [ ] Bounded Queue接続（timeout=5.0s）
- [ ] no_speech判定修正（VAD状態ベース）

**成功基準**:
- ✅ stdin読み取りがSTT遅延の影響を受けない（ユニットテスト合格）
- ✅ VAD状態ベースno_speech判定（偽no_speech率 <0.1%）

**ファイル**:
- `python-stt/main.py`（Thread model実装）
- `python-stt/stt_engine/audio_pipeline.py`（既存VAD活用）

---

### Phase 4: E2E Tests（0.5日）

**担当**: QA/両開発者
**期限**: Day 3

**タスク**:
- [ ] Test 1: 5秒Python停止 → 5秒後stop + error通知
- [ ] Test 2: 連続60秒発話 → フレームロス0
- [ ] Test 3: 偽no_speech抑止（VAD `is_in_speech()`中）
- [ ] Test 4: Sender/Receiver並行動作（ダミーPython）

**成功基準**:
- ✅ 全4テスト合格
- ✅ フレームドロップ率 = 0.0%
- ✅ Deadlock発生率 = 0%

**ファイル**:
- `src-tauri/tests/sidecar_full_duplex_e2e.rs`（新規）

---

## 成功基準（SLO）

### Functional Requirements

- ✅ **Deadlock発生率**: 0%（120秒連続発話でも）
- ✅ **フレームロス率**: 0%（正常動作時）
- ✅ **偽no_speech率**: <0.1%（VAD `is_in_speech()`中）
- ✅ **Python異常検出時間**: 5秒以内（timeout即座通知）

### Performance Requirements

- ✅ **Audio callback処理時間**: <10μs（ring.push()のみ）
- ✅ **E2E latency**: <100ms（音声入力 → partial_text表示）
- ✅ **Memory overhead**: 160 KB（Ring Buffer）+ 数百KB（queues）
- ✅ **CPU overhead**: <5%（3スレッド合計、idle時）

### Reliability Requirements

- ✅ **MTBF**: >24時間連続動作
- ✅ **Graceful degradation**: Python crash時に即座復旧通知
- ✅ **既存テスト合格**: Rust 26 + Python 143（全合格維持）

---

## リスクと緩和策

### Risk 1: JSON Overhead (~40%)

**Likelihood**: 🟢 N/A（決定的）
**Impact**: 🟢 Low（45 KB/s増加、現代システムでは無視可能）

**緩和策**: 不要（シンプルさとのトレードオフで許容）

---

### Risk 2: Python Queue.Full() Timeout False Positives

**Likelihood**: 🟡 Medium（一時的なCPUスパイク）
**Impact**: 🟡 Medium（誤エラー通知）

**緩和策**:
- timeout=5.0s設定（Rust 5秒バッファと同期）
- 詳細メトリクスログ（queue size、STT latency）
- 将来: 履歴ベース適応的timeout調整

---

### Risk 3: Whisper GIL Non-Release

**Likelihood**: 🟢 Low（多くのC++ STTライブラリはGIL解放）
**Impact**: 🔴 High（stdin reader停止）

**緩和策**:
- WhisperライブラリドキュメントでGIL解放確認
- Fallback: `asyncio.to_thread()`で強制スレッドプール実行
- stdin read latencyメトリクス監視

---

## 比較: ADR-013 vs ADR-011/012

| 項目                | ADR-011/012                    | ADR-013（本決定）                      | 改善点                 |
| ----------------- | ------------------------------ | --------------------------------- | ------------------- |
| **API設計**         | stdin/stdout独立Mutex（露出あり）       | AudioSink/EventStream facade（隠蔽）  | ✅ よりクリーンなAPI       |
| **Framing**       | 不明確                            | Line-Delimited JSON               | ✅ Deadlock回避明示      |
| **Buffer容量**      | 500 frames (5秒)                | 500 frames (5秒)                   | 同じ                  |
| **Overflow戦略**    | try_send() + UI通知              | try_send() + 即座停止 + UI通知          | ✅ よりクリアなUX         |
| **Python Model**  | 不明確（スレッド構成未定義）                 | Reader/VAD/STT分離（明示）              | ✅ 実装可能性向上          |
| **実装工数**          | 3-4日                           | **3日**                            | ✅ 0.5-1日短縮         |
| **ドキュメント完全性**     | 部分的（ADR-011/012のみ）             | **完全**（ADR-013 + 本決定書）            | ✅ 実装詳細まで含む         |
| **既存コードへの影響**     | 中（構造体変更 + 全呼び出し箇所修正）           | **小**（新API追加 + 段階移行可能）            | ✅ リスク低減            |

**結論**: ADR-013は**ADR-011/012の改良版**であり、実装工数・リスク・完全性のすべてで優位

---

## 関連ドキュメント

- **ADR-013**: Sidecar Full-Duplex IPC Final Design（本決定のベース）
- **ADR-008**: Dedicated Session Task (Rejected)
- **ADR-009**: Sender/Receiver Concurrent Architecture (Rejected)
- **ADR-011**: IPC Stdin/Stdout Mutex Separation (Superseded by ADR-013)
- **ADR-012**: Audio Callback Backpressure Redesign (Superseded by ADR-013)
- **Design Section 7.9**: IPC Protocol Architecture（ADR-013反映が必要）
- **Task 7.3**: IPC Deadlock Resolution（ADR-013反映が必要）

---

## Next Actions

### Immediate（本日中）

- [x] ADR-013作成
- [x] 本決定書作成
- [ ] spec.json更新準備（BLOCK-004追加: ADR-013実装待ち）
- [ ] Phase 1実装開始（Sidecar分離API）

### Short-term（3日以内）

- [ ] Phase 1-4実装完了
- [ ] E2E Tests全合格
- [ ] spec.json更新: phase=tasks-approved, BLOCK-004解消

### Long-term（1週間以内）

- [ ] 実装検証レポート作成
- [ ] ADR-008/009/011/012 → ADR-013移行完了宣言
- [ ] design.md Section 7.9更新（ADR-013準拠）
- [ ] tasks.md Task 7.3更新（ADR-013準拠）
- [ ] Production deployment準備

---

## 承認記録

- [x] Technical Review: 致命的欠陥2件修正確認済み（2025-10-14）
- [x] Critical Analysis: 代替案比較完了（2025-10-14）
- [ ] Implementation Lead: 実装可能性確認待ち
- [ ] Product Owner: UX影響確認待ち

---

**Document Version**: v1.0
**作成日**: 2025-10-14
**最終更新**: 2025-10-14
**ステータス**: ✅ **承認済み - 実装準備完了**
