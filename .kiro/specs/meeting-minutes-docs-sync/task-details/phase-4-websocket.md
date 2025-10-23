# Phase 4: WebSocket Protocol Extension (Week 4)

> **親ドキュメント**: [tasks.md](../tasks.md) | [task-details/README.md](README.md)
> **関連設計**: [design-data.md#WebSocket Extension](../design-modules/design-data.md) | [design-architecture.md#Offscreen Document](../design-modules/design-architecture.md)
> **Requirements**: DOCS-REQ-007.1-5

## Goal

WebSocketプロトコル拡張とMV3対応。`docsSync`フィールド追加、Tauri側メッセージ受信、Offscreen Document実装、WebSocketポート動的検出。

---

### 9. WebSocketメッセージ拡張の実装

_Requirements: DOCS-REQ-007.1-5_

#### 9.1 WebSocketメッセージ形式の拡張

WebSocketメッセージに`docsSync`フィールドを追加する。

**受け入れ基準**:
- [ ] Chrome拡張側: `docsSync`フィールドを含むメッセージ送信機能
- [ ] イベントタイプ: `docs_sync_started`, `docs_sync_success`, `docs_sync_error`, `docs_sync_offline`, `docs_sync_online`
- [ ] メッセージスキーマ検証（TypeScript型定義）
- [ ] ユニットテスト: メッセージシリアライゼーション/デシリアライゼーション

**技術詳細**:
- スキーマ: [design-data.md#SyncEvent](design-modules/design-data.md) L436-462参照
- 送信元: `SyncManager`
- 送信先: WebSocketサーバー（Tauriアプリ）

#### 9.2 Tauriアプリ側のメッセージ受信ロジック実装

_Requirements: DOCS-REQ-007.3-4_

Tauriアプリ側でWebSocketメッセージを受信し、UIに反映する機能を実装する。

**受け入れ基準**:
- [ ] WebSocketメッセージ受信ハンドラーの実装
- [ ] `docsSync`フィールドの存在チェック
- [ ] 同期ステータスの状態管理（Vuex/Pinia等）
- [ ] UI更新（同期中/成功/失敗のバッジ表示）
- [ ] 統合テスト: Chrome拡張 → Tauriアプリへのイベント送信

**技術詳細**:
- ファイル: `src-tauri/src/websocket/message_handler.rs`
- UI更新: `SyncStatusStore` (Vuex/Pinia)

#### 9.3 SyncStateStoreの実装（Tauri側）

_Requirements: DOCS-REQ-007.5, Design v1.3 Critical Fix_

Tauri側でChrome拡張の同期状態を管理するストアを実装する。

**受け入れ基準**:
- [ ] 同期状態管理（オンライン/オフライン、ドキュメントID、キュー数）
- [ ] `enrich_message()`関数の実装（`docsSync`フィールド合成）
- [ ] UIコンポーネントとの連携（Vue.js/React）
- [ ] 統合テスト: WebSocketメッセージ受信 → ストア更新 → UI反映

**技術詳細**:
- ファイル: `src/store/syncState.ts`
- 詳細: [design-data.md#SyncStateStore](design-modules/design-data.md) L464-509参照

### 10. Offscreen Document実装（MV3対応）

_Requirements: DOCS-REQ-007.2, Design v1.3 Critical Fix_

#### 10.1 Offscreen Documentライフサイクル管理の実装

WebSocket接続を維持するOffscreen Documentを実装する。

**受け入れ基準**:
- [ ] `chrome.offscreen.createDocument()`での生成
- [ ] `ensureOffscreenDocument()`パターンの実装
- [ ] Chrome再起動時の自動再生成（`chrome.runtime.onStartup`）
- [ ] 拡張インストール/更新時の自動再生成（`chrome.runtime.onInstalled`）
- [ ] 統合テスト: Chrome再起動 → Offscreen再生成 → WebSocket再接続

**技術詳細**:
- ファイル: `extension/background.js`, `extension/offscreen.html`, `extension/offscreen.js`
- 詳細: [design-architecture.md#Offscreen Document](design-modules/design-architecture.md) L118-163参照

#### 10.2 WebSocketポート動的検出の実装

_Requirements: DOCS-REQ-007.2, Design v1.3 Critical Fix_

meeting-minutes-coreの仕様（9001-9100の動的割り当て）に準拠したポートスキャンを実装する。

**受け入れ基準**:
- [ ] ポート範囲: 9001-9100（100ポート）
- [ ] 10ポートずつチャンク実行（CPU配慮）
- [ ] タイムアウト: 500ms/ポート
- [ ] **ポートキャッシング戦略（CRITICAL - レイテンシ要件達成に必須）**:
  - [ ] 最後に成功したポート番号を`chrome.storage.local`に保存（キー: `last_successful_websocket_port`）
  - [ ] 次回接続時は保存されたポートを最優先で試行（タイムアウト500ms）
  - [ ] キャッシュポート失敗時のみフルスキャン実行（初回接続または動的ポート変更時）
  - [ ] パフォーマンステスト: キャッシュヒット率90%以上で平均接続時間2秒以内を確認
- [ ] 統合テスト: Tauriアプリ起動 → ポートスキャン → 接続成功 → 再接続時にキャッシュ使用

**技術詳細**:
- 実装場所: `offscreen.js` `connectToTauriWebSocket()`
- **レイテンシ分析**:
  - 初回接続（キャッシュなし）: 平均25秒（50%位置）、最悪50秒
  - キャッシュヒット時: 0.5秒
  - キャッシュヒット率90%想定: 平均2.95秒 → ギリギリ許容範囲
  - **キャッシング実装が必須** - なければレイテンシ要件（2秒以内）を満たせない
- 詳細: [design-architecture.md#WebSocket Port Discovery](design-modules/design-architecture.md) L172-223参照

### 11. Phase 4検証とロールバック準備

#### 11.1 Phase 4検証チェックリストの実行

**受け入れ基準**:
- [ ] WebSocketメッセージに`docsSync`フィールドが含まれる
- [ ] Tauriアプリでイベントが正しく受信される
- [ ] Offscreen Documentが正常に動作する
- [ ] WebSocket接続がService Workerタイムアウトに影響されない
- [ ] 統合テストカバレッジ80%以上

#### 11.2 Phase 4ロールバック戦略の準備

**受け入れ基準**:
- [ ] WebSocketメッセージ形式の元への復元機能
- [ ] Offscreen Documentの無効化フラグ
- [ ] Phase 3の状態へのロールバック手順書

---

