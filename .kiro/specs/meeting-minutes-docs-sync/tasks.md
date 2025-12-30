# Implementation Tasks - meeting-minutes-docs-sync

**Feature**: OAuth 2.0 + Google Docs API統合でGoogle Meetから議事録を自動同期（MVP2）

**Phase**: tasks-generated

**Language**: ja

---

## Overview

本機能は **Phase 0（設計検証）+ Phase 1-5（実装）** の6フェーズに分けて段階的に実装します（[design-migration-appendix.md](design-modules/design-migration-appendix.md)参照）:

| Phase | Duration | Focus | Status |
|-------|----------|-------|--------|
| [Phase 0](task-details/phase-0-design-validation.md) | Week 0 | **設計検証・スケルトン実装** | ✅ 完了 |
| [Phase 1](task-details/phase-1-authentication.md) | Week 1 | OAuth 2.0認証レイヤー | ✅ 完了（getAuthToken移行） |
| [Phase 2](task-details/phase-2-api-integration.md) | Week 2 | Google Docs API統合 | ✅ 完了（Integration Test 7/7合格） |
| [Phase 3](task-details/phase-3-offline-sync.md) | Week 3 | オフライン/自動再同期 | ✅ 完了（全コンポーネント実装済み） |
| [Phase 4](task-details/phase-4-websocket.md) | Week 4 | WebSocketプロトコル拡張 | ✅ 完了（Offscreen Document + docsSync） |
| [Phase 5](task-details/phase-5-testing-release.md) | Week 5 | E2E/UAT/リリース | 🔵 次フェーズ |

---

## 📍 CURRENT STATUS: Phase 5開始

**Phase 0-4完了。Phase 5（E2E/UAT/リリース）を開始します。**

### 完了した実装（2025-12-29）

#### Phase 1: Authentication Layer ✅
- `chrome.identity.getAuthToken()` による認証（PKCE/client_secret不要）
- AuthManager簡素化（TokenRefresher/TokenStore依存削除）
- Popup UI（認証ボタン、状態表示）

#### Phase 2: API Integration ✅
- GoogleDocsClient（get/batchUpdate）
- ExponentialBackoffHandler（Jitter付き）
- OptimisticLockHandler（revisionId検証）
- NamedRangeManager（作成/取得/更新/復旧）
- ParagraphStyleFormatter（見出し/タイムスタンプ/話者名）
- **Integration Test 7/7合格**

#### Phase 3: Offline Sync ✅
- QueueManager（オフラインキュー管理）
- SyncManager（同期制御）
- TokenBucketRateLimiter（60 tokens/min）
- NetworkMonitor / StorageMonitor
- BufferingManager / ResyncOrchestrator
- **128テスト合格**

#### Phase 4: WebSocket Extension ✅
- Offscreen Document（WebSocket接続維持）
- ポートスキャン（9001-9100、キャッシング付き）
- docsSync メッセージ形式（started/success/error/offline/online/queue_update）
- Background↔Offscreen メッセージング
- Tauri側 docsSync受信・UI表示

### 次のステップ

1. **Phase 5開始**:
   - `task-details/phase-5-testing-release.md` を開く
   - E2Eテスト、パフォーマンステスト、セキュリティテスト、UAT

詳細なタスクナビゲーションは [task-details/README.md](task-details/README.md) を参照してください。

---

## Phase 0: Design Validation & Skeleton Implementation (Week 0) ⭐ NEW

**Goal**: 実装前の詳細設計検証とスケルトン実装生成

**Key Deliverables**:
- ドメイン別クラス図（Auth/Sync/API Domain、計19クラス）
- 責務マトリクス（全クラスの単一責務定義）
- インターフェース契約定義（事前条件/事後条件/エラー型）
- スケルトン実装（全19クラスの空実装）
- テストスケルトン（全19クラスのテストファイル + `it.todo()`）

**Requirements**: 全要件（設計検証フェーズ）

**Validation Checkpoints**:
- [ ] 全19クラスのクラス図承認
- [ ] 責務マトリクス承認（テスト容易性⭐4以上が80%以上）
- [ ] インターフェース契約承認（全メソッドに事前条件/事後条件）
- [ ] スケルトン実装生成完了（全クラスがコンパイル成功）
- [ ] テストスケルトン生成完了（全テストファイルに`it.todo()`列挙）

**詳細**: [phase-0-design-validation.md](task-details/phase-0-design-validation.md)

**Design Artifacts**:

**Phase 0成果物（`.kiro/specs/meeting-minutes-docs-sync/design-artifacts/`）**:
- [Responsibility Matrix](design-artifacts/responsibility-matrix.md) - 全19クラスの責務定義（Task 0.9で検証）
- [Interface Contracts](design-artifacts/interface-contracts.md) - 全19インターフェースの契約定義（Task 0.2で完成）

**プロジェクト共有図版（`docs/uml/`）**:
- [Auth Domain Class Diagram](/docs/uml/meeting-minutes-docs-sync/cls/auth-domain.md) - 5クラス
- [Sync Domain Class Diagram](/docs/uml/meeting-minutes-docs-sync/cls/sync-domain.md) - 8クラス
- [API Domain Class Diagram](/docs/uml/meeting-minutes-docs-sync/cls/api-domain.md) - 6クラス

---

## Phase 1: Authentication Layer (Week 1)

**Goal**: Chrome拡張への認証レイヤー追加

**Key Deliverables**:
- AuthManagerコンポーネント（OAuth 2.0フロー）
- TokenStoreコンポーネント（トークン永続化）
- トークンリフレッシュ機能（`chrome.alarms`使用）
- Popup UI「Google連携」ボタン

**Requirements**: DOCS-REQ-001.1-9, DOCS-NFR-003.1, DOCS-NFR-003.3

**Validation Checkpoints**:
- [x] OAuth 2.0認証フローが正常に動作する（getAuthToken移行）
- [x] トークンがChrome管理で保存される（getAuthToken）
- [x] トークンリフレッシュが正常に動作する（Chrome自動管理）
- [x] ユニットテストカバレッジ80%以上（128件合格）
- [ ] セキュリティ警告が表示される（Phase 5で実装）

**詳細**: [phase-1-authentication.md](task-details/phase-1-authentication.md)

---

## Phase 2: API Integration (Week 2)

**Goal**: Google Docs API統合とNamed Range管理

**Key Deliverables**:
- GoogleDocsClientコンポーネント（API呼び出し、Exponential Backoff、楽観ロック）
- NamedRangeManagerコンポーネント（挿入位置管理、自動復旧）
- 段落スタイル設定機能（見出し、タイムスタンプ、話者名）

**Requirements**: DOCS-REQ-002.1-13, DOCS-REQ-003.1-8, DOCS-REQ-006.1-6, DOCS-NFR-001.2

**Validation Checkpoints**:
- [x] Google Docs APIへのリクエストが成功する（Integration Test合格）
- [x] Named Rangeが正しく作成される（Integration Test合格）
- [x] テキストが正しい位置に挿入される（Integration Test合格）
- [x] エラーハンドリングが正常に動作する（Exponential Backoff/楽観ロック）
- [x] 統合テストカバレッジ80%以上（128件合格）

**詳細**: [phase-2-api-integration.md](task-details/phase-2-api-integration.md)

---

## Phase 3: Offline Sync (Week 3)

**Goal**: オフラインキューと自動再同期

**Key Deliverables**:
- QueueManagerコンポーネント（オフラインキュー管理、ストレージ監視）
- SyncManagerコンポーネント（同期制御、バッファリング戦略）
- Token Bucket Rate Limiter（60リクエスト/分遵守）
- 自動再同期機能（ネットワーク復帰時）

**Requirements**: DOCS-REQ-004.1-9, DOCS-REQ-005.1-12, DOCS-NFR-001.1-4

**Validation Checkpoints**:
- [x] オフライン時にメッセージがキューに保存される（QueueManager実装済み）
- [x] ネットワーク復帰時に自動再同期が実行される（ResyncOrchestrator実装済み）
- [x] ストレージ使用量の警告が表示される（StorageMonitor実装済み）
- [x] レート制限が遵守される（TokenBucketRateLimiter 60 tokens/min）
- [x] 統合テストカバレッジ80%以上（128件合格）

**詳細**: [phase-3-offline-sync.md](task-details/phase-3-offline-sync.md)

---

## Phase 4: WebSocket Extension (Week 4)

**Goal**: WebSocketプロトコル拡張とMV3対応

**Key Deliverables**:
- WebSocketメッセージ形式拡張（`docsSync`フィールド追加）
- Tauriアプリ側のメッセージ受信ロジック
- SyncStateStoreの実装（Tauri側）
- Offscreen Document実装（MV3 Service Worker対応）
- WebSocketポート動的検出（9001-9100スキャン）

**Requirements**: DOCS-REQ-007.1-5

**Validation Checkpoints**:
- [ ] WebSocketメッセージに`docsSync`フィールドが含まれる
- [ ] Tauriアプリでイベントが正しく受信される
- [ ] Offscreen Documentが正常に動作する
- [ ] WebSocket接続がService Workerタイムアウトに影響されない
- [ ] 統合テストカバレッジ80%以上

**詳細**: [phase-4-websocket.md](task-details/phase-4-websocket.md)

---

## Phase 5: Testing & Release (Week 5)

**Goal**: E2Eテストとユーザー受け入れテスト

**Key Deliverables**:
- E2Eテストスイート（6シナリオ: 認証、リアルタイム同期、オフライン復帰、Named Range復旧、レート制限、トークンリフレッシュ）
- パフォーマンステスト（4項目: 挿入2秒以内、API応答3秒以内、再同期120秒以内、ストレージ10ms以内）
- セキュリティテスト（トークンストレージ、HTTPS通信、Authorization Header、CSP）
- ユーザー設定機能（同期有効/無効、タイムスタンプ、話者名、バッファリング時間）
- ドキュメント作成（ユーザーマニュアル、開発者ドキュメント）
- ユーザー受け入れテスト（UAT）実施

**Requirements**: 全要件の検証、DOCS-REQ-008.1-5

**Validation Checkpoints**:
- [ ] 全E2Eテストが成功する
- [ ] ユーザー受け入れテストが完了する
- [ ] パフォーマンス目標を達成する
- [ ] セキュリティテストが成功する
- [ ] ドキュメントが完成する

**詳細**: [phase-5-testing-release.md](task-details/phase-5-testing-release.md)

---

## Success Criteria

本MVP2実装は、以下の条件を全て満たした場合に成功とみなされます（[requirements.md#Success Criteria](requirements.md#L351-361)参照）:

1. ✅ **OAuth 2.0認証**: Chrome拡張からGoogleアカウントにログインし、OAuth 2.0トークンを取得できる
2. ✅ **リアルタイム同期**: 文字起こし結果がリアルタイム（2秒以内）でGoogle Docsに反映される
3. ✅ **Named Range管理**: 文字起こし結果が構造化されたフォーマットでドキュメントに挿入される
4. ✅ **オフライン対応**: ネットワーク切断時もローカルキューに保存され、再接続時に自動同期される
5. ✅ **エラーハンドリング**: トークンリフレッシュ、APIエラー、ネットワークエラーに対して適切に対処する
6. ✅ **ユーザー設定**: Google Docs同期の有効/無効、タイムスタンプ表示等の設定が可能

---

## Rollback Triggers

以下の条件を満たす場合、ロールバックを実行します（[design-migration-appendix.md#Rollback Triggers](design-modules/design-migration-appendix.md#L113-121)参照）:

1. **認証失敗率が50%以上**: OAuth 2.0認証が頻繁に失敗する
2. **API呼び出し成功率が80%未満**: Google Docs API呼び出しが頻繁に失敗する
3. **オフラインキューの保存失敗率が10%以上**: ストレージ書き込みが頻繁に失敗する
4. **クリティカルなセキュリティ脆弱性の発見**: トークン漏洩やXSS攻撃のリスク

---

## Dependencies

### Upstream Dependencies (Blocking)

本specの実装開始前に、以下の成果物が完了している必要があります:

- **meeting-minutes-core** (phase: design-validated以降):
  - **CORE-REQ-006**: WebSocketサーバー (ポート9001-9100)
  - **CORE-REQ-007**: Chrome拡張スケルトン (WebSocket接続機能)
- **meeting-minutes-stt** (phase: implementation-completed):
  - **STT-REQ-008**: WebSocketメッセージ拡張 (confidence, language, isPartial フィールド)

### External Dependencies

- **Google Docs API**: v1
- **Google OAuth 2.0**: Google Identity Services
- **Chrome Extensions API**: Manifest V3
- **Chrome Storage API**: chrome.storage.local

詳細: [requirements.md#Dependencies](requirements.md#L364-390)

---

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-10-24 | 1.0 | Claude Code | 初版作成（タスク生成） |
| 2025-10-24 | 1.1 | Claude Code | tasks.mdを高レベル概要に変更、詳細タスクをtask-details/に分割 |
