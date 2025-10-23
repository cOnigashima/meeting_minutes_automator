# Phase 3: Offline Queue & Auto-Resync (Week 3)

> **親ドキュメント**: [tasks.md](../tasks.md) | [task-details/README.md](README.md)
> **関連設計**: [design-components.md#Sync Domain](../design-modules/design-components.md) | [design-flows.md#Offline Queue](../design-modules/design-flows.md)
> **Requirements**: DOCS-REQ-004.1-9, DOCS-REQ-005.1-12, DOCS-NFR-001.1-4

## Goal

オフラインキューと自動再同期。ネットワーク切断時のローカルキュー保存、レート制限対応、自動再同期機能の実装。

---

### 6. オフラインキュー管理の実装

_Requirements: DOCS-REQ-005.1-12, DOCS-NFR-001.4_

#### 6.1 QueueManagerコンポーネントの実装

オフライン時のメッセージキューイング機能を実装する。

**受け入れ基準**:
- [ ] `chrome.storage.local`への`offline_queue`保存機能
- [ ] メッセージのenqueue/dequeue機能（FIFO順序）
- [ ] タイムスタンプによるメッセージソート機能
- [ ] ストレージ使用量監視機能（`chrome.storage.local.getBytesInUse()`）
- [ ] ユニットテスト: キュー操作のカバレッジ80%以上

**技術詳細**:
- ファイル: `extension/src/sync/QueueManager.ts`
- ストレージキー: `offline_queue`
- ストレージ上限: 10 MB（QUOTA_BYTES_PER_ITEM）
- インターフェース: [design-components.md#QueueManager](design-modules/design-components.md) L274-340参照

#### 6.2 ストレージ使用量監視とアラート機能の実装

_Requirements: DOCS-REQ-005.11-12_

ストレージ使用量を定期監視し、警告を表示する機能を実装する。

**受け入れ基準**:
- [ ] **Offscreen Document上の`setInterval`での定期監視（6秒間隔）** - chrome.alarmsは最小1分制約のため
- [ ] 80%到達時のポップアップ警告表示
- [ ] 100%到達時の全画面通知（`chrome.notifications`）
- [ ] 上限到達時のメッセージ受信停止機能
- [ ] 統合テスト: ストレージ上限到達 → 警告表示 → 受信停止

**技術詳細**:
- 実装場所: `extension/offscreen.html` + `extension/offscreen.js`
- タイマー管理: `setInterval(() => checkStorageUsage(), 6000)` in Offscreen Document
- Service Workerとの通信: `chrome.runtime.sendMessage({ type: 'STORAGE_WARNING', usageRatio })`
- **理由**: chrome.alarmsは最小1分制約があり、6秒間隔の監視には使用不可
- 詳細: [design-testing-security.md#Storage Monitoring](design-modules/design-testing-security.md) L307-341参照

### 7. 同期制御機能の実装

_Requirements: DOCS-REQ-004.1-9, DOCS-REQ-005.5-10_

#### 7.1 SyncManagerコンポーネントの実装

オンライン/オフライン状態の管理と同期制御機能を実装する。

**受け入れ基準**:
- [ ] 同期開始機能（`startSync()`）
- [ ] 文字起こしメッセージ処理機能（`processTranscription()`）
- [ ] オンライン/オフライン状態の自動検知（`navigator.onLine`）
- [ ] 状態遷移管理（Stopped → Starting → OnlineSync ⇄ OfflineQueue → Resyncing）
- [ ] ユニットテスト: 状態遷移のカバレッジ80%以上

**技術詳細**:
- ファイル: `extension/src/sync/SyncManager.ts`
- 状態管理: [design-components.md#SyncManager](design-modules/design-components.md) L158-258参照
- 状態永続化: `chrome.storage.local` (`sync_status` key)

#### 7.2 バッファリング戦略の実装

_Requirements: DOCS-REQ-004.6-7, Design v1.3 Performance Optimization_

レート制限を遵守するためのバッファリング機能を実装する。

**受け入れ基準**:
- [ ] 最大バッファ時間: 3秒（**Offscreen Document上の`setInterval`使用** - chrome.alarmsは最小1分制約のため）
- [ ] 最大バッファサイズ: 500文字
- [ ] Offscreen Document上での自動フラッシュ（MV3対応）
- [ ] 複数メッセージの1回の`batchUpdate`へのマージ
- [ ] パフォーマンステスト: API呼び出し回数60%削減確認

**技術詳細**:
- 実装場所: `extension/offscreen.html` + `extension/offscreen.js`
- タイマー管理: `setInterval(() => flushBuffer(), 3000)` in Offscreen Document
- Service Workerとの通信: `chrome.runtime.sendMessage()` / `chrome.runtime.onMessage`
- **理由**: chrome.alarmsは最小1分制約があり、3秒間隔のフラッシュには使用不可
- 詳細: [design-testing-security.md#Buffering Strategy](design-modules/design-testing-security.md) L244-302参照

#### 7.3 自動再同期機能の実装

_Requirements: DOCS-REQ-005.5-10, DOCS-NFR-001.3_

ネットワーク復帰時の自動再同期機能を実装する。

**受け入れ基準**:
- [ ] ネットワーク復帰イベントの検知（`online`イベント）
- [ ] オフラインキューの取得とタイムスタンプ順ソート
- [ ] レート制限遵守（60リクエスト/分）
- [ ] 再同期進捗の表示（ポップアップUI）
- [ ] 統合テスト: ネットワーク切断 → キュー保存 → 復帰 → 自動再同期

**技術詳細**:
- 実装場所: `SyncManager.resyncOfflineQueue()`
- パフォーマンス目標: 100メッセージあたり最大120秒（DOCS-NFR-001.3）

#### 7.4 Token Bucket Rate Limiterの実装

_Requirements: DOCS-NFR-001.1-2, Design v1.3 Rate Limiting_

Google Docs APIレート制限（60リクエスト/分）を遵守するRate Limiterを実装する。

**受け入れ基準**:
- [ ] Token Bucket アルゴリズム実装
- [ ] 容量: 60 tokens/min、リフィル: 1 token/sec
- [ ] API呼び出し前のトークン取得機能（`acquire()`）
- [ ] トークン不足時の自動待機機能
- [ ] パフォーマンステスト: 429エラー発生率0%確認

**技術詳細**:
- ファイル: `extension/src/sync/TokenBucketRateLimiter.ts`
- 詳細: [design-testing-security.md#Token Bucket RateLimiter](design-modules/design-testing-security.md) L172-241参照

### 8. Phase 3検証とロールバック準備

#### 8.1 Phase 3検証チェックリストの実行

**受け入れ基準**:
- [ ] オフライン時にメッセージがキューに保存される
- [ ] ネットワーク復帰時に自動再同期が実行される
- [ ] ストレージ使用量の警告が表示される
- [ ] レート制限が遵守される（60リクエスト/分以下）
- [ ] 統合テストカバレッジ80%以上

#### 8.2 Phase 3ロールバック戦略の準備

**受け入れ基準**:
- [ ] オフラインキュー機能の無効化フラグ
- [ ] オンライン同期のみでの動作確認
- [ ] Phase 2の状態へのロールバック手順書

---

