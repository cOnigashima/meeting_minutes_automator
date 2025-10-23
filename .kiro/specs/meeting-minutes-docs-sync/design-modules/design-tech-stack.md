# Technical Design - meeting-minutes-docs-sync: Technology Stack

> **プロジェクト**: OAuth 2.0 + Google Docs API統合でGoogle Meetから議事録を自動同期（MVP2）
> **親ドキュメント**: [design.md](../design.md)
> **関連**: [Requirements](../requirements.md) | [Tasks](../tasks.md) | [他のモジュール](README.md)

## Technology Stack and Design Decisions

### Technology Alignment

meeting-minutes-docs-syncは、既存のChrome拡張技術スタックを拡張します:

**既存技術スタック（meeting-minutes-core, meeting-minutes-stt）**:
- **Chrome Extension Manifest V3**: Service Worker、Content Scripts、Popup UI
- **WebSocket**: `chrome.runtime.sendMessage` / `chrome.runtime.onMessage`
- **Storage**: `chrome.storage.local`（設定、セッション状態）

**新規導入ライブラリ**:
- **Chrome Identity API**: `chrome.identity.launchWebAuthFlow()`（OAuth 2.0認証フロー）
- **Fetch API**: Google Docs API通信（HTTPSリクエスト）
- **Chrome Notifications API**: `chrome.notifications.create()`（オフラインキュー上限通知）

**技術選択の根拠**:
- **Chrome Identity API**: Chrome拡張に最適化されたOAuth 2.0実装。リダイレクトURI処理を自動化
- **Fetch API**: ブラウザ標準API。`async/await`による可読性の高い非同期処理
- **chrome.storage.local**: Service Workerで利用可能な永続ストレージ。トークンとオフラインキューを保存

### Key Design Decisions

#### Decision 1: OAuth 2.0スコープの選択 - 必要最小限のスコープ組み合わせ

**Decision**: `documents` + `drive.file` スコープの組み合わせを使用する

**Context**: Google Docs API `batchUpdate`メソッドは、ドキュメントへの書き込み権限を必要とする。公式ドキュメントの検証により、`drive.file`のみでは不十分であることが判明した。

**Alternatives**:
1. **`documents`のみ**: 全Google Docsドキュメントへの完全アクセス（Sensitive Scope）
2. **`drive.file`のみ**: アプリで作成/共有されたファイルのみ（Non-sensitive）だが、Docs API書き込みには不足
3. **`documents` + `drive.file`**: Docs書き込み権限 + ファイルアクセス制限の組み合わせ（推奨）
4. **`drive`**: 全ファイルへの完全アクセス（Restricted Scope - 最も強い制限）

**Selected Approach**: **`documents` + `drive.file` スコープの組み合わせ**

```json
{
  "scopes": [
    "https://www.googleapis.com/auth/documents",
    "https://www.googleapis.com/auth/drive.file"
  ]
}
```

**Rationale**:
- **Google Docs API要件**: `documents`スコープはGoogle Docs API `batchUpdate`の書き込みに必須
- **最小権限の原則**: `drive.file`と組み合わせることで、アプリが作成/共有されたファイルのみにアクセスを制限
- **ユーザー信頼性**: 許可画面で明確な説明（「Google Docsの編集」+「アプリで作成したファイルのみ」）

**Trade-offs**:
- **得られるもの**: Google Docs API書き込み権限、ファイルアクセス制限、機能性の保証
- **失うもの**: `documents`はSensitive Scopeのため、OAuth同意画面で明確な説明が必要

**重要な注意事項**:
- `documents`スコープはSensitiveカテゴリのため、OAuth同意画面でユーザーに「Google Docsドキュメントの閲覧・編集・作成・削除」と表示される
- ユーザーに対して、アプリが「アクティブなドキュメントのみ」にアクセスすることを明確に説明する必要がある

---

#### Decision 2: Named Range管理戦略 - 自動再作成ロジック

**Decision**: Named Range (`transcript_cursor`) 消失時に、段階的フォールバック戦略で自動再作成する

**Context**: ユーザーがGoogle Docs上でNamed Rangeを手動削除、または他のツールが削除した場合、文字起こしの挿入位置が不明になる。

**Alternatives**:
1. **エラー停止**: Named Range消失時に同期を停止し、ユーザーに手動設定を促す
2. **ドキュメント末尾固定**: 常にドキュメントの末尾に追記する（構造化フォーマット無視）
3. **段階的フォールバック**: 複数の検出ロジックを試行し、最適な位置に自動再作成

**Selected Approach**: **段階的フォールバック戦略**

```typescript
async function recoverNamedRange(documentId: string): Promise<number> {
  // Priority 1: Search for "## 文字起こし" heading
  const headingIndex = await findHeadingIndex(documentId, "## 文字起こし");
  if (headingIndex !== null) {
    return headingIndex + 1; // Insert after heading
  }

  // Priority 2: Document end
  const doc = await getDocument(documentId);
  return doc.body.content[doc.body.content.length - 1].endIndex - 1;

  // Priority 3 (edge case): Empty document
  return 1; // Start of document
}
```

**Rationale**:
- **ユーザー体験**: エラーで停止せず、自動復旧により作業継続性を保証
- **構造化維持**: 「## 文字起こし」見出しを検索することで、構造化フォーマットを尊重
- **ログと通知**: ERRORレベルログ + UI通知により、ユーザーに異常を認識させる

**Trade-offs**:
- **得られるもの**: 高い可用性、ユーザーの手間削減、構造化フォーマット維持
- **失うもの**: 予期しない位置への挿入リスク（ログと通知で緩和）

---

#### Decision 3: オフラインキュー管理 - Storage上限対策と警告システム

**Decision**: オフラインキューに2段階の警告システムを実装し、上限到達時は新規メッセージの受信を停止する

**Context**: `chrome.storage.local`のデフォルト上限は10MBで、長時間のオフライン状態では文字起こしメッセージが蓄積しストレージ溢れのリスクがある。

**Alternatives**:
1. **無制限許可要求**: `unlimitedStorage`パーミッションを要求（ユーザーからの信頼低下リスク）
2. **古いメッセージを削除**: FIFOで古いメッセージを削除（データロスのリスク）
3. **2段階警告 + 受信停止**: 80%で警告、100%で新規受信停止とユーザー通知

**Selected Approach**: **2段階警告 + 受信停止**

```typescript
// DOCS-REQ-005.11: 80%到達時の警告
if (queueSize >= MAX_QUEUE_SIZE * 0.8) {
  showPopupWarning(`オフラインキューが残り${MAX_QUEUE_SIZE - queueSize}件です。ネットワーク接続を確認してください`);
}

// DOCS-REQ-005.12: 100%到達時の全画面通知
if (queueSize >= MAX_QUEUE_SIZE) {
  chrome.notifications.create({
    type: 'basic',
    iconUrl: 'icon.png',
    title: 'オフラインキュー上限到達',
    message: 'これ以上の文字起こしは保存されません。録音を停止するか、ネットワーク接続を回復してください'
  });
  stopReceivingMessages();
}
```

**Rationale**:
- **データ完全性**: データロスを回避し、ユーザーに明確な選択肢を提示
- **ユーザー制御**: 録音停止かネットワーク復旧の選択権をユーザーに委ねる
- **段階的劣化**: 突然の停止ではなく、事前警告により心理的負担を軽減

**Trade-offs**:
- **得られるもの**: データ完全性、ユーザーの意思決定サポート、予測可能な動作
- **失うもの**: 長時間オフライン時の自動継続性（設計上の制約として許容）

---

