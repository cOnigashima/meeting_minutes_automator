# Task 10.4 Phase 2 Final Implementation Design

**Document Version**: 1.4
**Created**: 2025-10-21
**Last Updated**: 2025-10-21
**Status**: Design Complete (Implementation Pending)
**Purpose**: Task 10.4 Phase 2（デバイス切断/再接続E2E）の最終実装設計書

---

## Executive Summary

Task 10.4 Phase 2は**9回の外部批判的レビュー**を経て、最終的な設計に到達した。本ドキュメントは、全レビューで指摘された致命的欠陥と、それに対する最終設計を記録する。

**最終アーキテクチャ（v1.4）:**
- JobState + Supervisor pattern
- AbortHandle使用（**tokio 1.47.1安定版API、tokio_unstable不要**）
- attempt情報・キャンセル理由の共有機構（Arc<AtomicU32>, Arc<Mutex<CancelReason>>）
- **CancelReason優先度制御**（NewJob > UserRequest > UserManualResume、高優先度は上書き不可）
- **ReconnectionResult::Cancelledにreason保持**（協調的終了・強制終了両方で理由伝達）
- take()しない&mut abort()方式
- 協調的キャンセル（cancel_flag）+ 強制停止（AbortHandle）の二段構え

**検証済み設計要素:**
- ✅ tokio_unstable不要（実コンパイル・実行確認済み）
- ✅ Arc<AtomicU32>によるattempt追跡（abort時も取得可能）
- ✅ Arc<Mutex<CancelReason>>による理由伝達（UserRequest/UserManualResume/NewJob区別）
- ✅ **CancelReason優先度制御**（NewJob最優先、上書きバグ根本解決）
- ✅ **ReconnectionResult::Cancelledにreason: Option<CancelReason>追加**（UI通知で理由区別可能）
- ✅ &mut戦略の一貫性（supervisor cleanup競合なし）
- ✅ cancel_flag必要性（AbortHandleと補完、100ms以内キャンセル検知）

**実装状況:**
- 設計完成（v1.4）
- 既知の制限なし（v1.3.1までの上書きバグをv1.4で解決）
- 実装チェックリスト準備済み（更新版）
- 次フェーズ: reconnection_manager.rs実装

---

## 外部レビュー履歴

### Review #1 (2025-10-20)
**指摘:** 8個の致命的欠陥
1. is_recording未更新
2. 初回5秒wait省略
3. 状態corruption
4. Race condition（clone pattern）
5. Send constraint違反
6. Resource leak
7. Sleep cancellation hole
8. FakeAudioDevice corruption

**対応:** Job-based architecture採用

---

### Review #2 (2025-10-20)
**指摘:** 4個の追加リスク
1. Deadlock（15秒ロック保持）
2. Self-deadlock
3. Timing issue

**対応:** ロック保持時間を数マイクロ秒に短縮

---

### Review #3 (2025-10-20)
**指摘:** 3個の致命的欠陥
1. Deadlock（lock held during loop）
2. Timing issue（user restart）
3. 誤通知

**対応:** Supervisor pattern提案受け入れ

---

### Review #4 (2025-10-21)
**指摘:** 2個の致命的欠陥
1. current_jobのcleanup機構不在
2. abort()時のクリーンアップ未実行

**対応（誤り）:** abort()完全廃止 → 協調的キャンセルのみ

**結果:** 新たな致命的欠陥を導入（7-10秒待機、並走競合）

---

### Review #5 (2025-10-21)
**指摘:** 5個の致命的欠陥
1. 協調キャンセルで7-10秒待機（STT-REQ-004.11違反）
2. 旧/新ジョブ並走競合
3. コメント/コード矛盾
4a. JoinHandle除外で停止不能
4b. current_job.take()後のcleanup失敗
5. attempt情報喪失
6. キャンセル理由不明

**対応（本設計）:** abort()復活 + 情報共有機構

---

### Review #6 (2025-10-21)
**指摘:** 5個の残存リスク
1. **tokio_unstable要件**: AbortHandleはtokio_unstableを有効にしない限り存在しない（→検証により**誤り**と判明）
2. **attempt/cancel_reason未実装**: Arc<AtomicU32>/Arc<Mutex<CancelReason>>が設計のみで実装なし
3. **take() vs &mut不整合**: 設計とコード例の矛盾
4. **cancel_flag用途曖昧**: AbortHandleとの併用で中途半端なパス増加（→**却下**、多重防御として必要）
5. **設計ドキュメント乖離**: reconnection_manager.rsと設計の完全乖離

**対応（v1.2）:**
- tokio_unstable検証: JoinHandle::abort_handle()は安定版API（実コンパイル・実行で確認）
- cancel_reason引数追加: reconnect_taskに引数追加、UserManualResume設定2箇所追加
- 指摘4却下: cancel_flagは協調的キャンセル（100ms以内）とAbortHandle（強制停止）の二段構え
- 実装は次フェーズ: 設計完成、実装チェックリスト（L458-467）に従い実装予定

---

### Review #7 (2025-10-21)
**指摘:** 2個の致命的欠陥
1. **手動再開時の理由欠落**: reconnect_taskがcancel_reasonを受け取っていない（→v1.2で**既に修正済み**と判明）
2. **ReconnectionResultが理由を保持していない**: Arc<Mutex<CancelReason>>を導入したが、最終結果に含まれない（→**致命的欠陥**、設計の根本的矛盾）

**問題分析（指摘2）:**
- **2つの終了パス:**
  1. 協調的終了: reconnect_taskがcancel_flag検知 → `Ok(Cancelled)` → supervisorはcancel_reason参照しない
  2. 強制終了: abort() → `Err(is_cancelled)` → supervisorがcancel_reason取得 → **でもReconnectionResultに含めない**
- **結果**: UI通知でキャンセル理由が区別できない（UserRequest vs UserManualResume vs NewJob）

**対応（v1.3）:**
- ReconnectionResult::Cancelledに`reason: Option<CancelReason>`フィールド追加
- reconnect_task内の3箇所でcancel_reason取得してreasonに含めてreturn:
  1. L443: cancel_flagチェック（UserRequest or NewJob）
  2. L454: ユーザー手動再開チェック（UserManualResume）
  3. L483: 5秒待機後のcancelledチェック（UserManualResume or UserRequest or NewJob）
- supervisor内でreason使用:
  1. L287: abort検知時にreasonを含める
  2. L355-367: UI通知時にreasonを文字列変換（user_cancel/user_manual_resume/new_disconnect_event/unknown）

---

### Review #8 (2025-10-21)
**指摘:** 3個の問題
1. **コード断片が古い形のまま**: `return ReconnectionResult::Cancelled { device_id, attempt }`でreasonなし（→**誤認**、v1.3で既に修正済み）
2. **emit_result仕様不透明**: UI通知の詳細仕様が記述されていない（→**誤認**、v1.3で詳細記述済み）
3. **cancel_flag再検討**: 理由上書きリスク、余計な分岐増加（→**部分的に正しい**、稀なケースで影響限定的）

**検証結果:**
- **指摘1-2は完全な誤認**: v1.3で既に対応済み
  - L443/L454/L483: 全てのreturn文がreasonを含む
  - L355-367: UI通知の詳細仕様（reason → reason_str変換、Tauriイベント構築）
- **指摘3は部分的に正しい**: cancel_reason上書きの可能性
  - シナリオ: NewJob設定後、5秒以内にUserManualResume上書き
  - 発生確率: 低い
  - 影響: 限定的（UI通知の理由文字列が変わるだけ）

**対応（v1.3.1）:**
- 「既知の制限」セクション追加: cancel_reason上書きシナリオを文書化
- 設計変更なし: 実害 < 設計変更コスト

---

### Review #9 (2025-10-21)
**指摘:** 3個の問題
1. **cancel_reason上書きは致命的**: 「稀」ではない、トラブルシューティング時に真逆の情報を表示（→**正しい**）
2. **cancel_flag責務があいまい**: 3経路で共有、上書きバグの原因（→**部分的に正しい**、完全分離は時期尚早）
3. **emit_result仕様不明**: `emit_result(&app_super, result);`としか書かれていない（→**誤認**、v1.3で詳細記述済み）

**検証結果:**
- **指摘1は正しい**: 上書きは頻繁に発生しうる、影響は致命的
  - 発生確率再評価: 5秒待機時間は十分長い、ユーザーが録音再開押すのは自然
  - 影響再評価: 因果関係が逆転 → デバッグ不能
  - v1.3.1の「稀なケース」判断は**誤り**
- **指摘2は部分的に正しい**: 責務分離は有効だが、完全廃止は問題あり
  - cancel_flag廃止 → 100ms遅延発生の可能性
  - 代替案: 責務分離（設計変更コスト大）
  - 判断: 優先度制御で本質的問題を解決、責務分離は将来の改善課題
- **指摘3は誤認**: v1.3で既にL355-367で詳細記述済み
  - レビュアーがv1.2以前を見ている可能性

**対応（v1.4）:**
- **CancelReason優先度制御を実装**:
  - NewJob(3) > UserRequest(2) > UserManualResume(1)
  - 高優先度の理由は低優先度で上書き不可
  - `set_cancel_reason_priority()`ヘルパー関数追加
  - 全てのcancel_reason設定箇所でヘルパー使用
- **既知の制限セクション削除**: 優先度制御で根本解決

---

## 設計原則（Final）

1. ✅ **Immediate cancellation**: AbortHandle.abort()で即座停止
2. ✅ **Guaranteed cleanup**: Supervisor内でabort検知 + cleanup
3. ✅ **Information preservation**: attempt・キャンセル理由を共有
4. ✅ **Race-free**: job_id比較 + take()しないabort()
5. ✅ **STT-REQ-004.11準拠**: 即座のユーザー保護

---

## アーキテクチャ

### 全体構成

```
ReconnectionManager
  ├─ current_job: Option<JobState>
  ├─ next_job_id: AtomicU64

start_job()
  ├─ 旧ジョブabort（take()せず、&mutでabort_handleアクセス）
  ├─ reconnect_task起動
  │   └─ current_attempt更新（Arc<AtomicU32>）
  └─ supervisor起動（独立タスク）
       ├─ JoinHandle.awaitで完了待機
       ├─ abort検知 → current_attempt/cancel_reason取得
       ├─ job_id比較 + cleanup
       └─ UI通知（attempt・理由付き）
```

---

## データ構造

### JobState

```rust
struct JobState {
    /// Unique job identifier (for race-free cleanup)
    id: u64,

    /// Abort handle for immediate cancellation
    abort_handle: tokio::task::AbortHandle,

    /// Cooperative cancellation flag (100ms polling)
    cancel_flag: Arc<AtomicBool>,

    /// Current attempt number (shared with reconnect_task)
    current_attempt: Arc<AtomicU32>,

    /// Cancellation reason (None = not cancelled, Some = reason)
    cancel_reason: Arc<Mutex<Option<CancelReason>>>,

    /// Device ID being reconnected
    device_id: String,
}
```

**設計判断:**
- `AbortHandle`: tokio 1.19.0以降の安定版API（tokio_unstable不要）
- `current_attempt`: reconnect_taskとsupervisorで共有、abort時も取得可能
- `cancel_reason`: ユーザーキャンセル・手動再開・新ジョブを区別

---

### CancelReason

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CancelReason {
    /// User manually resumed recording (is_recording=true)
    /// 優先度: 低（他の理由で上書き可能）
    UserManualResume = 1,

    /// User called cancel_reconnection command
    /// 優先度: 中（NewJobのみ上書き可能）
    UserRequest = 2,

    /// New disconnect event triggered new job
    /// 優先度: 高（上書き不可）
    NewJob = 3,
}
```

**優先度制御:**
- NewJob(3) > UserRequest(2) > UserManualResume(1)
- 高優先度の理由は低優先度で上書きできない
- 例: NewJob設定後にUserManualResume検出しても上書きしない

**設計判断:**
- **NewJob最優先**: デバイス切断イベントの因果関係を保持（トラブルシューティング重視）
- **UserRequest中間**: ユーザー明示的操作を記録
- **UserManualResume最低**: 副次的な操作（録音再開はキャンセル理由として弱い）

---

### CancelReason設定ヘルパー

```rust
/// Cancel reason設定（優先度制御付き）
fn set_cancel_reason_priority(
    cancel_reason: &Arc<Mutex<Option<CancelReason>>>,
    new_reason: CancelReason,
) {
    let mut reason = cancel_reason.lock().unwrap();
    match *reason {
        None => {
            // 未設定なら無条件で設定
            *reason = Some(new_reason);
        }
        Some(existing) if new_reason > existing => {
            // 高優先度で上書き
            *reason = Some(new_reason);
        }
        _ => {
            // 低優先度では上書きしない（既存の理由を保持）
        }
    }
}
```

---

### ReconnectionResult

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ReconnectionResult {
    /// Reconnection successful
    Success { device_id: String, attempts: u32 },

    /// All retries exhausted
    Failed {
        device_id: String,
        attempts: u32,
        last_error: String,
    },

    /// Cancelled by user, manual resume, or new job
    Cancelled {
        device_id: String,
        attempt: u32,
        reason: Option<CancelReason>,  // ★ キャンセル理由
    },
}
```

**設計判断:**
- `reason: Option<CancelReason>`: キャンセル理由を結果に含める
  - `Some(UserRequest)`: ユーザーがcancel_reconnection呼び出し
  - `Some(UserManualResume)`: ユーザーが手動で録音再開
  - `Some(NewJob)`: 新しい切断イベントで旧ジョブ中断
  - `None`: 理由不明（想定外のパス、デバッグ用）
- Arc<Mutex<CancelReason>>と併用: reconnect_task終了時に理由を取得してReconnectionResultに含める

---

## 主要処理フロー

### 1. start_job() - 新ジョブ起動

```rust
pub fn start_job(&mut self, device_id: String, app: AppHandle) {
    // Step 1: 旧ジョブabort（take()しない）
    if let Some(old_job) = &mut self.current_job {
        set_cancel_reason_priority(&old_job.cancel_reason, CancelReason::NewJob);
        old_job.cancel_flag.store(true, Ordering::Relaxed);
        old_job.abort_handle.abort();  // ← 即座停止
        log_info_details!("previous_job_aborted", ...);
    }

    // Step 2: 新ジョブ準備
    let job_id = self.next_job_id.fetch_add(1, Ordering::SeqCst);
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let current_attempt = Arc::new(AtomicU32::new(0));
    let cancel_reason = Arc::new(Mutex::new(None));

    // Step 3: reconnect_task起動
    let handle = tokio::spawn(async move {
        reconnect_task(
            app.clone(),
            device_id.clone(),
            cancel_flag.clone(),
            current_attempt.clone(),
            cancel_reason.clone(),  // ★ 追加
        ).await
    });

    let abort_handle = handle.abort_handle();

    // Step 4: supervisor起動（handleを消費）
    let app_super = app.clone();
    let device_id_super = device_id.clone();
    let attempt_super = current_attempt.clone();
    let reason_super = cancel_reason.clone();

    tokio::spawn(async move {
        let result = match handle.await {
            Ok(result) => result,
            Err(e) if e.is_cancelled() => {
                // abort()検知 → attempt・理由取得
                let attempt = attempt_super.load(Ordering::Relaxed);
                let reason = reason_super.lock().unwrap().take();

                log_info_details!("task_aborted", json!({
                    "job_id": job_id,
                    "attempt": attempt,
                    "reason": format!("{:?}", reason)
                }));

                ReconnectionResult::Cancelled {
                    device_id: device_id_super.clone(),
                    attempt,
                    reason,  // ★ reason追加
                }
            }
            Err(e) => {
                // panic
                ReconnectionResult::Failed {
                    device_id: device_id_super.clone(),
                    attempts: 0,
                    last_error: format!("Task panicked: {:?}", e),
                }
            }
        };

        // Step 5: cleanup（abort時も実行）
        {
            let state = app_super.state::<AppState>();
            let mut mgr = state.reconnection_manager.lock().await;

            if mgr.current_job.as_ref().map(|j| j.id) == Some(job_id) {
                mgr.current_job = None;
                log_info_details!("job_cleaned_up", json!({ "job_id": job_id }));
            } else {
                log_info_details!("job_already_replaced", json!({
                    "job_id": job_id,
                    "current_job_id": mgr.current_job.as_ref().map(|j| j.id)
                }));
            }
        }

        // Step 6: UI通知（result.reasonを使用）
        match result {
            ReconnectionResult::Success { device_id, attempts } => {
                log_info_details!("reconnection::supervisor", "success",
                    json!({ "job_id": job_id, "device_id": device_id, "attempts": attempts }));
                let _ = app_super.emit("device_reconnect_success",
                    json!({ "device_id": device_id, "attempts": attempts }));
            }
            ReconnectionResult::Failed { device_id, attempts, last_error } => {
                log_error_details!("reconnection::supervisor", "failed",
                    json!({ "job_id": job_id, "device_id": device_id, "attempts": attempts, "error": last_error }));
                let _ = app_super.emit("device_reconnect_failed",
                    json!({ "device_id": device_id, "attempts": attempts, "error": last_error }));
            }
            ReconnectionResult::Cancelled { device_id, attempt, reason } => {
                // ★ reasonを使って詳細なUI通知
                let reason_str = match reason {
                    Some(CancelReason::UserRequest) => "user_cancel",
                    Some(CancelReason::UserManualResume) => "user_manual_resume",
                    Some(CancelReason::NewJob) => "new_disconnect_event",
                    None => "unknown",
                };
                log_info_details!("reconnection::supervisor", "cancelled",
                    json!({ "job_id": job_id, "device_id": device_id, "attempt": attempt, "reason": reason_str }));
                let _ = app_super.emit("device_reconnect_cancelled",
                    json!({ "device_id": device_id, "attempt": attempt, "reason": reason_str }));
            }
        }
    });

    // Step 5: JobState登録（take()した旧ジョブを上書き）
    self.current_job = Some(JobState {
        id: job_id,
        abort_handle,
        cancel_flag,
        current_attempt,
        cancel_reason,
        device_id: device_id.clone(),
    });

    log_info_details!("job_started", json!({ "job_id": job_id, "device_id": device_id }));
}
```

**重要ポイント:**
- `&mut self.current_job`でabort → take()しない
- `current_job = Some(...)`で旧ジョブ上書き → supervisorのcleanupと競合しない
- abort_handleはtokio安定版API

---

### 2. cancel() - ユーザーキャンセル

```rust
pub fn cancel(&mut self) {
    if let Some(job) = &mut self.current_job {  // ← &mut、take()しない
        set_cancel_reason_priority(&job.cancel_reason, CancelReason::UserRequest);
        job.cancel_flag.store(true, Ordering::Relaxed);
        job.abort_handle.abort();

        log_info_details!("cancelled_by_user", json!({
            "job_id": job.id,
            "device_id": job.device_id
        }));
    }
}
```

**重要ポイント:**
- take()せず、&mutでabort
- cancel_reasonを設定 → supervisor内で取得
- current_jobはSomeのまま → supervisor内のcleanupが正常動作

---

### 3. reconnect_task - 再接続ループ

```rust
async fn reconnect_task(
    app: AppHandle,
    device_id: String,
    cancel_flag: Arc<AtomicBool>,
    current_attempt: Arc<AtomicU32>,
    cancel_reason: Arc<Mutex<Option<CancelReason>>>,  // ★ 追加
) -> ReconnectionResult {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY: Duration = Duration::from_secs(5);

    for attempt in 1..=MAX_RETRIES {
        // ★ attempt更新（supervisorから参照可能）
        current_attempt.store(attempt, Ordering::Relaxed);

        log_info_details!("attempt_start", json!({
            "device_id": device_id,
            "attempt": attempt,
            "max_attempts": MAX_RETRIES
        }));

        // Step 1: cancel_flagチェック
        if cancel_flag.load(Ordering::Relaxed) {
            // ★ cancel_reasonを取得（UserRequest or NewJob）
            let reason = cancel_reason.lock().unwrap().take();
            return ReconnectionResult::Cancelled { device_id, attempt, reason };
        }

        // Step 2: ユーザー手動再開チェック
        {
            let state = app.state::<AppState>();
            let is_recording = state.is_recording.lock().unwrap();
            if *is_recording {
                // ★ cancel_reasonをUserManualResumeに設定（優先度制御付き）
                set_cancel_reason_priority(&cancel_reason, CancelReason::UserManualResume);
                let reason = cancel_reason.lock().unwrap().take();
                return ReconnectionResult::Cancelled { device_id, attempt, reason };
            }
        }

        // Step 3: 5秒待機（100msポーリング）
        let cancelled = tokio::select! {
            _ = tokio::time::sleep(RETRY_DELAY) => false,
            _ = async {
                loop {
                    tokio::time::sleep(Duration::from_millis(100)).await;

                    if cancel_flag.load(Ordering::Relaxed) {
                        break;
                    }

                    let state = app.state::<AppState>();
                    let is_recording = state.is_recording.lock().unwrap();
                    if *is_recording {
                        // ★ 5秒待機中のUserManualResume検出（優先度制御付き）
                        set_cancel_reason_priority(&cancel_reason, CancelReason::UserManualResume);
                        break;
                    }
                }
            } => true,
        };

        if cancelled {
            // ★ cancel_reasonを取得（UserManualResume or UserRequest or NewJob）
            let reason = cancel_reason.lock().unwrap().take();
            return ReconnectionResult::Cancelled { device_id, attempt, reason };
        }

        // Step 4: デバイス列挙
        // Step 5: start_recording_internal
        // ...（既存コードと同じ）
    }

    // All retries exhausted
    ReconnectionResult::Failed {
        device_id,
        attempts: MAX_RETRIES,
        last_error: "All retries exhausted".to_string(),
    }
}
```

---

## 解決される欠陥

| 欠陥 | 原因 | 解決方法 |
|------|------|---------|
| 7-10秒待機 | abort()廃止 | AbortHandle.abort()で即座停止 |
| 旧/新ジョブ並走 | abort()なし | start_job()で旧ジョブをabort() |
| attempt情報喪失 | JoinError::is_cancelled()のみ | Arc<AtomicU32>で共有 |
| キャンセル理由不明 | abort()のみ | Arc<Mutex<Option<CancelReason>>>で共有 |
| cleanup失敗 | current_job.take() | take()せず&mutでabort |
| コメント矛盾 | 設計変更後の更新漏れ | 設計原則コメント修正 |

---

## トレードオフ

### ✅ 採用した方針

**1. AbortHandle使用（tokio安定版）**
- tokio 1.19.0以降で安定版API
- tokio_unstable不要
- futures::future::Abortableより直接的

**2. 情報共有（Arc<AtomicU32> + Arc<Mutex<Option<CancelReason>>>）**
- reconnect_taskとsupervisor間でattempt・理由を共有
- abort()後も情報取得可能
- オーバーヘッド: 無視できるレベル

**3. take()しないabort()**
- current_jobを維持したままabort_handleアクセス
- supervisor内のcleanupが正常動作
- 競合なし

### ❌ 却下した代替案

**1. 協調的キャンセルのみ（Review #4対応）**
- 7-10秒待機 → STT-REQ-004.11違反
- 旧/新ジョブ並走 → 競合リスク

**2. futures::future::Abortable**
- tokio::AbortHandleで十分
- 追加依存不要

**3. HashMap + 世代カウンター**
- 実装複雑度増加
- job_id比較で十分

---

## テスト戦略

### ユニットテスト

1. **即時キャンセル:**
   - cancel()呼び出し → 200ms以内に停止確認
   - 手法: tokio::time::sleep(200ms) → is_reconnecting() == false

2. **attempt情報保持:**
   - 2回目のリトライ中にabort → attempt=2確認
   - 手法: supervisorログ確認

3. **cleanup確実性:**
   - abort時 → current_job=None確認
   - 自然終了時 → current_job=None確認

### E2Eテスト

1. **旧/新ジョブ並走防止:**
   - DeviceGone連続発生 → 旧ジョブ即座停止確認
   - 手法: ログで"task_aborted"イベント確認

2. **UI通知完全性:**
   - キャンセル理由・attempt情報確認
   - 手法: Tauriイベント受信確認

---

## 実装チェックリスト

### reconnection_manager.rs

- [ ] **CancelReason enum追加**:
  - [ ] `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]`
  - [ ] UserManualResume = 1, UserRequest = 2, NewJob = 3（優先度）
- [ ] **set_cancel_reason_priority()ヘルパー追加**:
  - [ ] 優先度制御ロジック実装
  - [ ] None → 無条件設定、Some → 優先度比較
- [ ] **ReconnectionResult enum修正**: Cancelled variantに`reason: Option<CancelReason>`追加
- [ ] **JobState構造体拡張**: abort_handle, current_attempt, cancel_reason追加
- [ ] **start_job()修正**:
  - [ ] `set_cancel_reason_priority(&old_job.cancel_reason, CancelReason::NewJob)`使用
  - [ ] &mut old_job.abort()（take()しない）
  - [ ] supervisor内でabort検知時にreason含めてCancelled構築
  - [ ] supervisor内UI通知でreasonを文字列変換（user_cancel/user_manual_resume/new_disconnect_event/unknown）
- [ ] **cancel()修正**:
  - [ ] `set_cancel_reason_priority(&job.cancel_reason, CancelReason::UserRequest)`使用
  - [ ] &mut job.abort()（take()しない）
- [ ] **reconnect_task修正**:
  - [ ] 引数追加: current_attempt, cancel_reason（2つ）
  - [ ] current_attempt.store()（ループ開始時）
  - [ ] cancel_flagチェック時にreason取得してCancelled返却
  - [ ] ユーザー手動再開チェック時に`set_cancel_reason_priority()`使用（2箇所）
- [ ] **設計原則コメント更新**

### テスト

- [ ] cargo test --lib → 76/76 passed（既存テスト維持）
- [ ] 即時キャンセルテスト（200ms以内停止確認）
- [ ] attempt情報確認（supervisorログ）
- [ ] **cancel_reason確認**（UI通知イベントで3種類のreasonを区別: user_cancel/user_manual_resume/new_disconnect_event）
- [ ] cleanup確認（current_job=None、supervisorログ）

---

## リスク分析

### 低リスク
- ✅ AbortHandle API安定
- ✅ AtomicU32/Mutex基本API
- ✅ supervisor設計維持

### 中リスク
- ⚠️ 情報共有オーバーヘッド
  - **対策:** AtomicU32・Mutexの読み書きは数ナノ秒

### 高リスク
- **なし**

---

## 既知の制限

### なし（v1.4で全て解決）

~~v1.3.1までの既知の制限（cancel_reason上書き）はv1.4の優先度制御で解決しました。~~

---

## 参考資料

- **tokio::task::AbortHandle**: https://docs.rs/tokio/latest/tokio/task/struct.AbortHandle.html
- **STT-REQ-004.11**: `.kiro/specs/meeting-minutes-stt/requirements.md`
- **外部レビュー履歴**: `phase-13-re-scoping-rationale.md` セクション10-11
- **ADR-013**: Sidecar Full-Duplex Final Design

---

## 設計検証結果（2025-10-21 Post-Review）

### 検証済み設計要素

#### ✅ tokio_unstable要件（外部レビュー指摘）
**指摘**: 「AbortHandleはtokio_unstableを有効にしない限り存在しない」

**検証結果**: **誤り**
- tokio 1.47.1公式ドキュメント確認: `JoinHandle::abort_handle()`は安定版API
- 実コンパイル・実行テスト: `tokio_unstable`フラグなしで正常動作
- 結論: AbortHandle使用に追加フィーチャーフラグ不要

#### ✅ Arc<AtomicU32> (current_attempt追跡)
**妥当性**: ✅ 必要
- abort時のattempt情報保持に必須
- 競合なし（reconnect_taskのみ書き込み、supervisorは読み取りのみ）
- オーバーヘッド: 数ナノ秒（無視可能）

#### ✅ take() vs &mut戦略
**一貫性**: ✅ 一貫している
- start_job: `&mut`でabort → `Some()`上書き
- cancel: `&mut`でabort → current_job維持
- supervisor cleanup: job_id比較で競合なし

#### ✅ cancel_flag必要性（AbortHandleとの関係）
**冗長性**: ❌ 冗長ではない（必要）
- AbortHandle: `.await`ポイントでのみキャンセル検知
- cancel_flag: 100msポーリング内で即座検知（L345）
- 協調的キャンセル（cancel_flag） + 強制停止（AbortHandle）の二段構え
- STT-REQ-004.11準拠（即座のユーザー保護）

#### ✅ Arc<Mutex<Option<CancelReason>>> + ReconnectionResult::Cancelled (理由伝達)
**妥当性**: ✅ 必要（v1.3で完全実装）

**修正内容（v1.2-1.3で反映）:**
1. **reconnect_task引数追加**: `cancel_reason: Arc<Mutex<Option<CancelReason>>>`を引数に追加
2. **UserManualResume設定**: ループ前チェックと5秒待機中チェックで設定（2箇所）
3. **ReconnectionResult::Cancelled拡張**: `reason: Option<CancelReason>`フィールド追加（v1.3）
4. **reconnect_task内でreason取得**: 3箇所のCancelled返却時にcancel_reason取得してreasonに含める
5. **supervisor内でreason使用**: UI通知時にreasonを文字列変換（user_cancel/user_manual_resume/new_disconnect_event/unknown）

**設計判断:**
- 必要性: UI通知で詳細理由を区別（UserRequest vs UserManualResume vs NewJob）
- 二段階設計:
  1. Arc<Mutex<CancelReason>>: reconnect_task内で理由を設定（外部から設定も可能）
  2. ReconnectionResult::Cancelled: 理由を最終結果に含める（協調的終了・強制終了両方で伝達）
- オーバーヘッド: Mutexアクセスはキャンセル時のみ（頻度低、影響小）

---

## 変更履歴

| 日付 | バージョン | 変更内容 |
|------|----------|---------|
| 2025-10-21 | 1.0 | 初版作成（Review #5対応設計） |
| 2025-10-21 | 1.1 | 設計検証結果追加（tokio_unstable検証、cancel_reason不備指摘） |
| 2025-10-21 | 1.2 | Review #6対応（cancel_reason引数追加、UserManualResume設定追加） |
| 2025-10-21 | 1.3 | Review #7対応（ReconnectionResult::Cancelledにreason追加、UI通知でreason区別） |
| 2025-10-21 | 1.3.1 | Review #8対応（既知の制限セクション追加：cancel_reason上書きリスク文書化） |
| 2025-10-21 | 1.4 | **Review #9対応（CancelReason優先度制御実装、上書きバグ根本解決）** |
