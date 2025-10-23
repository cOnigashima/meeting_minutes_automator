# Phase 1: Authentication Layer (Week 1)

> **親ドキュメント**: [tasks.md](../tasks.md) | [task-details/README.md](README.md)
> **関連設計**: [design-components.md#Auth Domain](../design-modules/design-components.md) | [design-flows.md#OAuth Flow](../design-modules/design-flows.md)
> **Requirements**: DOCS-REQ-001.1-9, DOCS-NFR-003.1, DOCS-NFR-003.3

## Goal

Chrome拡張への認証レイヤー追加。OAuth 2.0フローの実装とトークン管理機能の確立。

---

### 1. OAuth 2.0認証フローの実装

_Requirements: DOCS-REQ-001.1-7, DOCS-NFR-003.1, DOCS-NFR-003.3_

#### 1.1 AuthManagerコンポーネントの実装

Chrome Identity APIを使用したOAuth 2.0認証フローを実装する。

**受け入れ基準**:
- [ ] `chrome.identity.launchWebAuthFlow()`を使用して認証ダイアログを開く機能
- [ ] 認証コードをアクセストークン/リフレッシュトークンに交換する機能
- [ ] OAuth 2.0スコープ: `documents` + `drive.file`の使用
- [ ] エラーハンドリング（ユーザーキャンセル、ネットワークエラー、Invalid Grant）
- [ ] ユニットテスト: 正常系/異常系のカバレッジ80%以上

**技術詳細**:
- ファイル: `extension/src/auth/AuthManager.ts`
- 依存: Chrome Identity API、Google OAuth 2.0
- インターフェース: [design-components.md#AuthManager](design-modules/design-components.md) L72-155参照

#### 1.2 TokenStoreコンポーネントの実装

`chrome.storage.local`を使用したトークン永続化機能を実装する。

**受け入れ基準**:
- [ ] アクセストークン、リフレッシュトークン、有効期限の保存機能
- [ ] トークン取得/削除機能
- [ ] MVP2では暗号化なし（セキュリティ警告表示のみ、MVP3で暗号化実装予定）
- [ ] Service Workerサスペンド時の自動ログアウト機能
- [ ] ユニットテスト: ストレージ操作のカバレッジ80%以上

**技術詳細**:
- ファイル: `extension/src/auth/TokenStore.ts`
- 依存: `chrome.storage.local`
- セキュリティ: DOCS-NFR-003.1, アクセストークン有効期限30分

#### 1.3 トークンリフレッシュ機能の実装

_Requirements: DOCS-REQ-001.8-9_

アクセストークン期限切れ時の自動リフレッシュ機能を実装する。

**受け入れ基準**:
- [ ] 有効期限の60秒前に自動リフレッシュ（クロックスキュー対策）
- [ ] リフレッシュトークン無効時の再認証プロンプト表示
- [ ] エラーハンドリング（401 Unauthorized、ネットワークエラー）
- [ ] 統合テスト: 認証 → 期限切れ → 自動リフレッシュ → API呼び出し成功

**技術詳細**:
- 実装場所: `AuthManager.refreshToken()`
- タイマー管理: `chrome.alarms` API使用（MV3対応）

#### 1.4 Popup UIへの「Google連携」ボタン追加

_Requirements: DOCS-REQ-001.1-2, DOCS-NFR-005.1_

ユーザーがOAuth 2.0認証を開始できるUI要素を追加する。

**受け入れ基準**:
- [ ] Popup UIに「Google連携」ボタンを追加
- [ ] 認証状態の視覚的表示（未認証/認証済み/エラー）
- [ ] 認証フロー開始時のガイド表示
- [ ] 認証成功時の成功通知表示
- [ ] 「Google連携解除」ボタンの追加（トークン無効化 + ローカル削除）

**技術詳細**:
- ファイル: `extension/popup/popup.html`, `extension/popup/popup.ts`
- UI: 認証状態バッジ（緑: 認証済み、グレー: 未認証、赤: エラー）

### 2. Phase 1検証とロールバック準備

_Requirements: Phase 1完了基準_

#### 2.1 Phase 1検証チェックリストの実行

**受け入れ基準**:
- [ ] OAuth 2.0認証フローが正常に動作する
- [ ] トークンが`chrome.storage.local`に保存される
- [ ] トークンリフレッシュが正常に動作する
- [ ] ユニットテストカバレッジ80%以上
- [ ] セキュリティ警告が表示される

#### 2.2 Phase 1ロールバック戦略の準備

**受け入れ基準**:
- [ ] 機能フラグ実装（OAuth 2.0機能の有効/無効切り替え）
- [ ] `chrome.storage.local`からの認証状態リセットスクリプト
- [ ] ロールバック手順書の作成
