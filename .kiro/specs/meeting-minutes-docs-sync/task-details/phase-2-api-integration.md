## Phase 2: Google Docs API Integration (Week 2)

### 3. Google Docs APIクライアントの実装

_Requirements: DOCS-REQ-002.1-9, DOCS-NFR-001.2_

#### 3.1 GoogleDocsClientコンポーネントの実装

Google Docs API v1のラッパークライアントを実装する。

**受け入れ基準**:
- [ ] `documents.get` API呼び出し機能（ドキュメント取得）
- [ ] `documents.batchUpdate` API呼び出し機能（テキスト挿入）
- [ ] Authorization Headerへのアクセストークン設定（`Bearer {token}`）
- [ ] HTTPS通信の強制（DOCS-NFR-003.2）
- [ ] エラーハンドリング（401, 403, 404, 429, 500-504）
- [ ] ユニットテスト: API呼び出しのモック/スタブテスト

**技術詳細**:
- ファイル: `extension/src/api/GoogleDocsClient.ts`
- エンドポイント: `https://docs.googleapis.com/v1/documents`
- インターフェース: [design-components.md#GoogleDocsClient](design-modules/design-components.md) L434-520参照

#### 3.2 Exponential Backoffリトライ戦略の実装

_Requirements: DOCS-REQ-002.7-9, DOCS-NFR-001.2_

APIエラー時の自動リトライ機能を実装する。

**受け入れ基準**:
- [ ] リトライ可能エラー判定（408, 429, 500, 502, 503, 504）
- [ ] 指数バックオフ（初回1秒、最大60秒）
- [ ] Jitter（ランダム遅延）の追加
- [ ] 最大リトライ回数: 5回
- [ ] 統合テスト: 429エラー → Exponential Backoff → 成功

**技術詳細**:
- 実装場所: `GoogleDocsClient.exponentialBackoff()`
- アルゴリズム: [design-components.md](design-modules/design-components.md) L402-432参照

#### 3.3 楽観ロック（Optimistic Locking）の実装

_Requirements: DOCS-REQ-002.13, Design v1.3 Critical Fix_

複数タブ/共同編集者との競合を防ぐ楽観ロック機能を実装する。

**受け入れ基準**:
- [ ] `writeControl.requiredRevisionId`を使用した楽観ロック
- [ ] リビジョンミスマッチ検出（400エラー）
- [ ] 最大リトライ回数: 3回
- [ ] カーソル位置再計算（Named Rangeから取得）
- [ ] 統合テスト: 複数タブからの同時挿入 → 競合検出 → 自動リトライ

**技術詳細**:
- 実装場所: `GoogleDocsClient.insertTextWithLock()`
- 詳細: [design-components.md#Optimistic Locking](design-modules/design-components.md) L522-605参照

### 4. Named Range管理機能の実装

_Requirements: DOCS-REQ-003.1-8, DOCS-REQ-006.1-6_

#### 4.1 NamedRangeManagerコンポーネントの実装

Google Docs内の挿入位置を管理する機能を実装する。

**受け入れ基準**:
- [ ] `transcript_cursor` Named Rangeの作成機能
- [ ] Named Range位置取得機能
- [ ] Named Range位置更新機能（テキスト挿入後）
- [ ] ドキュメント構造管理（見出し、タイムスタンプ、話者名の挿入）
- [ ] ユニットテスト: Named Range操作のカバレッジ80%以上

**技術詳細**:
- ファイル: `extension/src/api/NamedRangeManager.ts`
- Named Range名: `transcript_cursor`
- インターフェース: [design-components.md#NamedRangeManager](design-modules/design-components.md) L607-699参照

#### 4.2 Named Range自動復旧ロジックの実装

_Requirements: DOCS-REQ-003.7-8_

ユーザーがNamed Rangeを削除した場合の自動復旧機能を実装する。

**受け入れ基準**:
- [ ] Priority 1: 見出し検索（「## 文字起こし」）→ 見出し直後に再作成
- [ ] Priority 2: ドキュメント末尾に再作成
- [ ] Priority 3: ドキュメント先頭（index=1）に再作成
- [ ] ERRORログ記録 + UI通知（ポップアップ）
- [ ] 統合テスト: Named Range削除 → 挿入試行 → 自動復旧 → 正常挿入

**技術詳細**:
- 実装場所: `NamedRangeManager.recoverNamedRange()`
- フロー: [design-flows.md#Named Range Recovery](design-modules/design-flows.md) L125-166参照

#### 4.3 段落スタイル設定機能の実装

_Requirements: DOCS-REQ-006.3-4_

見出しと本文テキストの段落スタイルを設定する機能を実装する。

**受け入れ基準**:
- [ ] 見出しスタイル: `HEADING_2`（14pt、太字）
- [ ] 本文スタイル: `NORMAL_TEXT`（11pt、通常）
- [ ] タイムスタンプフォーマット: `[HH:MM:SS]`
- [ ] 話者名フォーマット: `**[話者名]**: `（設定で有効化時）

**技術詳細**:
- API: `updateParagraphStyle` request
- スタイル定義: [design-data.md#ParagraphStyle](design-modules/design-data.md) L226-263参照

### 5. Phase 2検証とロールバック準備

#### 5.1 Phase 2検証チェックリストの実行

**受け入れ基準**:
- [ ] Google Docs APIへのリクエストが成功する
- [ ] Named Rangeが正しく作成される
- [ ] テキストが正しい位置に挿入される
- [ ] エラーハンドリングが正常に動作する
- [ ] 統合テストカバレッジ80%以上

#### 5.2 Phase 2ロールバック戦略の準備

**受け入れ基準**:
- [ ] Google Docs API機能の無効化フラグ
- [ ] オフラインキューモード単独動作確認
- [ ] Phase 1の状態へのロールバック手順書

---

