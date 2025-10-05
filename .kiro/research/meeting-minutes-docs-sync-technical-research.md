# Meeting Minutes Docs Sync - Technical Research Report

**作成日**: 2025-10-03
**対象機能**: meeting-minutes-docs-sync (MVP2)
**調査範囲**: Google Docs API v1, Chrome Extension OAuth 2.0, Chrome Storage API, エラーハンドリングパターン

---

## 目次

1. [Google Docs API v1 batchUpdate](#1-google-docs-api-v1-batchupdate)
2. [OAuth 2.0 for Chrome Extensions](#2-oauth-20-for-chrome-extensions)
3. [Chrome Storage API](#3-chrome-storage-api)
4. [エラーハンドリングパターン](#4-エラーハンドリングパターン)
5. [実装推奨事項](#5-実装推奨事項)

---

## 1. Google Docs API v1 batchUpdate

### 1.1 エンドポイント概要

**HTTP Request:**
```
POST https://docs.googleapis.com/v1/documents/{documentId}:batchUpdate
```

**必須パラメータ:**
- `documentId` (string): 更新対象のドキュメントID

**認証スコープ:**
```
https://www.googleapis.com/auth/documents         (Sensitive)
https://www.googleapis.com/auth/drive              (Restricted)
https://www.googleapis.com/auth/drive.file         (Non-sensitive) ⭐ 推奨
```

### 1.2 スコープの選択

| スコープ | アクセスレベル | 権限 | 推奨度 |
|---------|-------------|------|--------|
| `documents` | Sensitive | すべてのGoogle Docs文書の閲覧・編集・作成・削除 | △ |
| `documents.readonly` | Sensitive | すべてのGoogle Docs文書の閲覧のみ | - |
| `drive.file` | **Non-sensitive** | **このアプリで作成/明示的に共有されたファイルのみ** | ⭐ **推奨** |
| `drive` | Restricted | すべてのGoogle Driveファイルの閲覧・編集・作成・削除 | ✗ |
| `drive.readonly` | Restricted | すべてのGoogle Driveファイルの閲覧・ダウンロード | ✗ |

**推奨事項:**
- `drive.file` スコープを使用することで、Non-sensitiveカテゴリとなり、検証プロセスが簡素化される
- ユーザーが明示的に選択した文書のみにアクセスするため、セキュリティとプライバシーが向上
- すべてのDrive API REST Resourcesで使用可能

### 1.3 リクエスト/レスポンスフォーマット

**リクエストボディスキーマ:**
```json
{
  "requests": [
    { "insertText": {...} },
    { "createNamedRange": {...} },
    { "replaceNamedRangeContent": {...} }
  ],
  "writeControl": {
    "requiredRevisionId": "string"
  }
}
```

**重要な動作特性:**
- ✅ **原子性**: すべてのリクエストが一括で適用される（部分的な適用は行われない）
- ✅ **検証**: いずれかのリクエストが無効な場合、全体が失敗する
- ✅ **順序保証**: レスポンスの順序はリクエストの順序と一致する
- ⚠️ **コラボレーション**: 他のユーザーの編集により、インデックスがずれる可能性がある

### 1.4 主要なRequest型

#### 1.4.1 InsertTextRequest

文書の特定位置にテキストを挿入します。

**スキーマ:**
```json
{
  "insertText": {
    "location": {
      "index": 25
    },
    "text": "挿入するテキスト"
  }
}
```

**または、セグメント末尾への挿入:**
```json
{
  "insertText": {
    "endOfSegmentLocation": {
      "segmentId": "string"
    },
    "text": "挿入するテキスト"
  }
}
```

**コード例 (JavaScript):**
```javascript
await client.documents.batchUpdate({
  documentId: docId,
  requestBody: {
    requests: [{
      insertText: {
        location: { index: 1 },
        text: 'Meeting Minutes - 2025-10-03\n\n'
      }
    }]
  }
});
```

**コード例 (Python):**
```python
requests = [{
    'insertText': {
        'location': {'index': 25},
        'text': 'Discussion summary...'
    }
}]
result = service.documents().batchUpdate(
    documentId=DOCUMENT_ID,
    body={'requests': requests}
).execute()
```

#### 1.4.2 CreateNamedRangeRequest

名前付き範囲（Named Range）を作成します。Named Rangeは文書内の特定セクションへの参照を保持し、コンテンツの追加・削除に応じて自動的にインデックスが更新されます。

**スキーマ:**
```json
{
  "createNamedRange": {
    "name": "meeting_minutes_section",
    "range": {
      "segmentId": "string",
      "startIndex": 10,
      "endIndex": 50,
      "tabId": "string"
    }
  }
}
```

**重要な特性:**
- ✅ インデックスは自動更新される（コンテンツの挿入・削除時）
- ✅ 文書内の特定セクションを参照しやすくする
- ⚠️ Named Rangeはプライベートではなく、文書へのアクセス権を持つ全員が閲覧可能
- ⚠️ コンテンツをコピーした場合、Named Rangeは元のテキストに残る（コピー先には付与されない）

**コード例 (Python):**
```python
requests = [{
    'createNamedRange': {
        'name': 'discussion_section',
        'range': {
            'segmentId': segment_id,
            'startIndex': start,
            'endIndex': start + text_length,
            'tabId': tab_id
        }
    }
}]
```

**コード例 (Java):**
```java
requests.add(
    new Request()
        .setCreateNamedRange(
            new CreateNamedRangeRequest()
                .setName("action_items")
                .setRange(
                    new Range()
                        .setSegmentId(range.getSegmentId())
                        .setStartIndex(range.getStartIndex())
                        .setEndIndex(range.getStartIndex() + newText.length())
                        .setTabId(range.getTabId()))));
```

#### 1.4.3 DeleteNamedRangeRequest

名前付き範囲を削除します（コンテンツは削除されません）。

**スキーマ:**
```json
{
  "deleteNamedRange": {
    "namedRangeId": "string"
  }
}
```

**または名前で削除（同名のすべての範囲が削除される）:**
```json
{
  "deleteNamedRange": {
    "name": "meeting_minutes_section",
    "tabsCriteria": {
      "tabIds": ["tab1", "tab2"]
    }
  }
}
```

#### 1.4.4 ReplaceNamedRangeContentRequest

名前付き範囲内のコンテンツを置換します。

**スキーマ:**
```json
{
  "replaceNamedRangeContent": {
    "namedRangeId": "string",
    "text": "新しいコンテンツ",
    "tabsCriteria": {
      "tabIds": ["tab1"]
    }
  }
}
```

**または名前で置換:**
```json
{
  "replaceNamedRangeContent": {
    "namedRangeName": "discussion_section",
    "text": "更新されたディスカッション内容"
  }
}
```

**注意事項:**
- ⚠️ `updateNamedRange` というリクエスト型は存在しない
- Named Rangeのプロパティを更新する場合は、削除して再作成する必要がある

### 1.5 Named Rangeの使用パターン

**典型的なワークフロー:**

1. **文書の現在の状態を取得**
```javascript
const doc = await client.documents.get({ documentId: docId });
const namedRanges = doc.namedRanges;
```

2. **既存のNamed Rangeを検索**
```javascript
const discussionRange = namedRanges['discussion_section'];
if (discussionRange) {
  const { startIndex, endIndex } = discussionRange.namedRanges[0].ranges[0];
}
```

3. **既存コンテンツを削除して新しいテキストを挿入**
```javascript
const requests = [
  // 既存コンテンツを削除
  {
    deleteContentRange: {
      range: {
        startIndex: startIndex,
        endIndex: endIndex
      }
    }
  },
  // 新しいテキストを挿入
  {
    insertText: {
      location: { index: startIndex },
      text: newContent
    }
  },
  // Named Rangeを再作成（新しいインデックスで）
  {
    createNamedRange: {
      name: 'discussion_section',
      range: {
        startIndex: startIndex,
        endIndex: startIndex + newContent.length
      }
    }
  }
];
```

### 1.6 レート制限とクォータ

**Google Docs API のレート制限:**

| 制限種別 | 値 |
|---------|-----|
| **読み取りリクエスト** (プロジェクトごと/分) | 3,000 |
| **読み取りリクエスト** (ユーザーごと/プロジェクトごと/分) | 300 |
| **書き込みリクエスト** (プロジェクトごと/分) | 600 |
| **書き込みリクエスト** (ユーザーごと/プロジェクトごと/分) | 60 |

**クォータ超過時の挙動:**
- HTTPステータスコード: `429 Too Many Requests`
- 推奨対応: Exponential Backoffアルゴリズムによる再試行

**クォータ管理のベストプラクティス:**
- ✅ Google Cloud Consoleからクォータ調整をリクエスト可能
- ✅ クォータ超過による追加料金は発生しない
- ✅ プロジェクトの使用量増加に応じて自動的にクォータが増加する場合がある

### 1.7 エラーコード

Google Docs APIは標準的なHTTPステータスコードを使用します：

| コード | 意味 | 対応方法 |
|-------|-----|---------|
| 400 | Bad Request | リクエストパラメータを確認、入力を検証 |
| 401 | Unauthorized | アクセストークンを更新、再認証 |
| 403 | Forbidden | 権限確認、レート制限の場合は指数バックオフで再試行 |
| 404 | Not Found | ファイルの存在確認、アクセス権の確認 |
| 429 | Too Many Requests | 指数バックオフで再試行 |
| 500-504 | Server Error | 指数バックオフで再試行 |

---

## 2. OAuth 2.0 for Chrome Extensions

### 2.1 chrome.identity API 概要

Chrome拡張機能でOAuth 2.0認証を実装するための公式API。

**主要メソッド:**
- `chrome.identity.launchWebAuthFlow()`: 認証フローの開始
- `chrome.identity.getAuthToken()`: Google専用の簡易認証（今回は使用しない）

### 2.2 launchWebAuthFlow メソッド

**メソッドシグネチャ:**
```javascript
chrome.identity.launchWebAuthFlow(
  details: WebAuthFlowDetails
): Promise<string | undefined>
```

**WebAuthFlowDetails パラメータ:**

| パラメータ | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| `url` | string | ✅ | 認証フローの初期URL（OAuth 2.0 authorization endpoint） |
| `interactive` | boolean | - | `true`: ユーザーにサインインプロンプトを表示<br>`false`: サイレント認証（失敗する可能性あり） |
| `abortOnLoadForNonInteractive` | boolean | - | (Chrome 113+) `true` (デフォルト): ページロード後に非インタラクティブフローを終了 |
| `timeoutMsForNonInteractive` | number | - | (Chrome 113+) 非インタラクティブモードの最大実行時間（ミリ秒） |

**戻り値:**
- 成功時: リダイレクトURL（認証コードまたはアクセストークンを含む）
- 失敗時: `undefined` またはエラー

### 2.3 実装例

**基本的な認証フロー:**

```javascript
// OAuth 2.0パラメータを構築
const authParams = new URLSearchParams({
  client_id: 'YOUR_CLIENT_ID.apps.googleusercontent.com',
  redirect_uri: chrome.identity.getRedirectURL(),
  response_type: 'code',
  scope: 'https://www.googleapis.com/auth/drive.file',
  access_type: 'offline',  // refresh tokenを取得するため
  prompt: 'consent'         // 常に同意画面を表示（refresh token取得のため）
});

const authUrl = `https://accounts.google.com/o/oauth2/v2/auth?${authParams}`;

// 認証フローを開始
try {
  const redirectUrl = await chrome.identity.launchWebAuthFlow({
    url: authUrl,
    interactive: true
  });

  // リダイレクトURLから認証コードを抽出
  const url = new URL(redirectUrl);
  const code = url.searchParams.get('code');

  if (code) {
    // 認証コードをアクセストークンと交換
    await exchangeCodeForTokens(code);
  }
} catch (error) {
  console.error('Authentication failed:', error);
}
```

**トークン交換処理:**

```javascript
async function exchangeCodeForTokens(code) {
  const tokenParams = new URLSearchParams({
    code: code,
    client_id: 'YOUR_CLIENT_ID.apps.googleusercontent.com',
    client_secret: 'YOUR_CLIENT_SECRET',
    redirect_uri: chrome.identity.getRedirectURL(),
    grant_type: 'authorization_code'
  });

  const response = await fetch('https://oauth2.googleapis.com/token', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded'
    },
    body: tokenParams
  });

  if (response.ok) {
    const tokens = await response.json();
    // {
    //   access_token: "ya29.a0...",
    //   refresh_token: "1//0g...",
    //   expires_in: 3599,
    //   token_type: "Bearer"
    // }

    // トークンを保存
    const expiresAt = Math.floor(Date.now() / 1000) + tokens.expires_in;
    await chrome.storage.local.set({
      accessToken: tokens.access_token,
      refreshToken: tokens.refresh_token,
      expiresAt: expiresAt
    });

    return tokens;
  } else {
    throw new Error('Token exchange failed');
  }
}
```

### 2.4 トークン管理

#### 2.4.1 トークンリフレッシュ

アクセストークンは有効期限（通常3600秒 = 1時間）があるため、定期的にリフレッシュする必要があります。

**リフレッシュ実装例:**

```javascript
async function getValidAccessToken() {
  const items = await chrome.storage.local.get([
    'accessToken',
    'refreshToken',
    'expiresAt'
  ]);

  const nowInSeconds = Math.floor(Date.now() / 1000);
  const nowPlus60 = nowInSeconds + 60;  // 60秒のバッファを設ける（クロックスキュー対策）

  // トークンが60秒以内に期限切れになる場合はリフレッシュ
  if (items.expiresAt <= nowPlus60) {
    const newTokens = await refreshAccessToken(items.refreshToken);
    return newTokens.access_token;
  }

  return items.accessToken;
}

async function refreshAccessToken(refreshToken) {
  const params = new URLSearchParams({
    client_id: 'YOUR_CLIENT_ID.apps.googleusercontent.com',
    client_secret: 'YOUR_CLIENT_SECRET',
    refresh_token: refreshToken,
    grant_type: 'refresh_token'
  });

  const response = await fetch('https://oauth2.googleapis.com/token', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded'
    },
    body: params
  });

  if (response.ok) {
    const tokens = await response.json();
    // {
    //   access_token: "ya29.a0...",
    //   expires_in: 3599,
    //   token_type: "Bearer"
    // }
    // 注意: refresh_tokenは含まれない（既存のものを再利用）

    const expiresAt = Math.floor(Date.now() / 1000) + tokens.expires_in;
    await chrome.storage.local.set({
      accessToken: tokens.access_token,
      expiresAt: expiresAt
    });

    return tokens;
  } else {
    // リフレッシュトークンが無効または取り消された場合
    throw new Error('Refresh token invalid');
  }
}
```

#### 2.4.2 API呼び出し時のトークン使用

```javascript
async function callGoogleDocsAPI(documentId, requests) {
  const accessToken = await getValidAccessToken();

  const response = await fetch(
    `https://docs.googleapis.com/v1/documents/${documentId}:batchUpdate`,
    {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${accessToken}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ requests })
    }
  );

  if (!response.ok) {
    if (response.status === 401) {
      // トークンが無効な場合は再認証
      await chrome.storage.local.remove(['accessToken', 'refreshToken', 'expiresAt']);
      throw new Error('Authentication required');
    }
    throw new Error(`API call failed: ${response.status}`);
  }

  return await response.json();
}
```

### 2.5 セキュリティとベストプラクティス

#### 2.5.1 トークン保存のセキュリティ

**⚠️ 重要な制限事項:**
Chrome Storage APIは暗号化されていません。「機密ユーザー情報を保存すべきではない」と公式ドキュメントに明記されています。

**推奨事項:**
- ✅ Chrome Storage APIを使用する（拡張機能の標準的な方法）
- ✅ トークンを絶対にプレーンテキストで送信しない
- ✅ 不要になったトークンは即座に削除する
- ✅ トークンの有効期限を適切に管理する
- ✅ ユーザーがサインアウトした場合は即座にトークンを削除
- ⚠️ より強固なセキュリティが必要な場合は、トークンをサーバー側で管理することを検討

**トークン削除（サインアウト）の実装:**

```javascript
async function signOut() {
  const items = await chrome.storage.local.get(['accessToken']);

  if (items.accessToken) {
    // トークンを取り消す
    await fetch(`https://oauth2.googleapis.com/revoke?token=${items.accessToken}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded'
      }
    });
  }

  // ローカルストレージから削除
  await chrome.storage.local.remove(['accessToken', 'refreshToken', 'expiresAt']);
}
```

#### 2.5.2 ユーザーエクスペリエンスのベストプラクティス

1. **インタラクティブ認証のタイミング:**
   - ✅ UIからユーザーアクションに応じて開始する
   - ✅ 認証の目的を明確に説明する
   - ✗ アプリ起動時に自動的に認証フローを開始しない

2. **`interactive` パラメータの使い分け:**
   - `interactive: true`: 初回認証、トークン期限切れ時
   - `interactive: false`: バックグラウンドでのトークンリフレッシュ（サイレント認証）

3. **エラーハンドリング:**
   - ユーザーが認証をキャンセルした場合の適切な処理
   - ネットワークエラーの場合のリトライロジック
   - トークン取り消し時の再認証フロー

### 2.6 manifest.json の設定

**必須設定:**

```json
{
  "manifest_version": 3,
  "name": "Meeting Minutes Automator",
  "permissions": [
    "identity",
    "storage"
  ],
  "host_permissions": [
    "https://docs.googleapis.com/*",
    "https://oauth2.googleapis.com/*"
  ],
  "oauth2": {
    "client_id": "YOUR_CLIENT_ID.apps.googleusercontent.com",
    "scopes": [
      "https://www.googleapis.com/auth/drive.file"
    ]
  }
}
```

**注意事項:**
- `oauth2` セクションは `getAuthToken()` メソッド用（今回は使用しない可能性あり）
- `launchWebAuthFlow()` を使用する場合、`oauth2` セクションは不要
- `identity` パーミッションは必須

---

## 3. Chrome Storage API

### 3.1 chrome.storage.local 概要

**特徴:**
- ✅ 各マシンにローカルに保存される
- ✅ コンテンツスクリプトからデフォルトでアクセス可能
- ✅ ブラウザの履歴削除では消えない（Web Storage APIとの違い）
- ✅ Service Workerで使用可能（localStorageは使用不可）
- ⚠️ 暗号化されていない

### 3.2 ストレージ制限

| 項目 | 値 |
|-----|-----|
| **デフォルト制限** | 10 MB (Chrome 114+)<br>5 MB (Chrome 113以前) |
| **unlimitedStorage権限使用時** | 無制限 |
| **計算方法** | すべてのキーの長さ + すべての値のJSON文字列化後のサイズ |

**制限超過時の挙動:**
- 更新が即座に失敗する
- コールバック使用時: `runtime.lastError` が設定される
- Promise使用時: Promiseがrejectされる

### 3.3 基本的な使用方法

#### 3.3.1 データの保存

```javascript
// 単一の値を保存
await chrome.storage.local.set({ key: value });
console.log("Value is set");

// 複数の値を保存
await chrome.storage.local.set({
  accessToken: "ya29.a0...",
  refreshToken: "1//0g...",
  expiresAt: 1696348800
});
```

#### 3.3.2 データの取得

```javascript
// 単一の値を取得
const result = await chrome.storage.local.get(["key"]);
console.log("Value is " + result.key);

// 複数の値を取得
const items = await chrome.storage.local.get([
  "accessToken",
  "refreshToken",
  "expiresAt"
]);
console.log(items.accessToken);

// すべての値を取得
const allItems = await chrome.storage.local.get(null);
```

#### 3.3.3 データの削除

```javascript
// 特定のキーを削除
await chrome.storage.local.remove(["accessToken", "refreshToken"]);

// すべてのデータをクリア
await chrome.storage.local.clear();
```

### 3.4 オフラインキュー管理の実装

Meeting Minutes Automatorでは、オフライン時に同期できなかった議事録をキューに保存する必要があります。

#### 3.4.1 キューデータ構造

```javascript
// キューアイテムの構造
interface QueueItem {
  id: string;              // ユニークID (UUID)
  timestamp: number;       // 作成タイムスタンプ
  documentId: string;      // Google DocsのドキュメントID
  operation: 'append' | 'update' | 'create';
  requests: Array<any>;    // Google Docs API batchUpdate requests
  retryCount: number;      // 再試行回数
  lastError?: string;      // 最後のエラーメッセージ
}
```

#### 3.4.2 キューの追加

```javascript
async function addToSyncQueue(item) {
  // 既存のキューを取得
  const { syncQueue = [] } = await chrome.storage.local.get(['syncQueue']);

  // 新しいアイテムを追加
  const queueItem = {
    id: crypto.randomUUID(),
    timestamp: Date.now(),
    ...item,
    retryCount: 0
  };

  syncQueue.push(queueItem);

  // キューを保存
  await chrome.storage.local.set({ syncQueue });

  console.log(`Added to sync queue: ${queueItem.id}`);
  return queueItem.id;
}
```

#### 3.4.3 キューの処理

```javascript
async function processSyncQueue() {
  const { syncQueue = [] } = await chrome.storage.local.get(['syncQueue']);

  if (syncQueue.length === 0) {
    console.log('Sync queue is empty');
    return;
  }

  console.log(`Processing ${syncQueue.length} items in sync queue`);

  const failedItems = [];

  for (const item of syncQueue) {
    try {
      // Google Docs APIにリクエストを送信
      await callGoogleDocsAPI(item.documentId, item.requests);
      console.log(`Successfully synced: ${item.id}`);
    } catch (error) {
      console.error(`Failed to sync ${item.id}:`, error);

      item.retryCount++;
      item.lastError = error.message;

      // 最大リトライ回数を超えた場合は削除、それ以外は再キュー
      if (item.retryCount < 3) {
        failedItems.push(item);
      } else {
        console.error(`Max retries exceeded for ${item.id}, removing from queue`);
      }
    }
  }

  // 失敗したアイテムのみを保存
  await chrome.storage.local.set({ syncQueue: failedItems });
}
```

#### 3.4.4 ストレージ使用量の監視

```javascript
async function checkStorageUsage() {
  // 使用量を取得（バイト単位）
  const bytesInUse = await chrome.storage.local.getBytesInUse(null);
  const mbInUse = (bytesInUse / (1024 * 1024)).toFixed(2);

  console.log(`Storage usage: ${mbInUse} MB`);

  // 制限に近い場合は警告
  const limit = 10 * 1024 * 1024; // 10 MB
  if (bytesInUse > limit * 0.8) {
    console.warn('Storage usage is over 80% of limit');
    // 古いキューアイテムを削除するなどの処理
  }

  return bytesInUse;
}
```

### 3.5 ベストプラクティス

1. **ストレージの選択:**
   - `chrome.storage.local`: 大量のデータ、ローカルのみで必要なデータ
   - `chrome.storage.sync`: ユーザー設定、小規模データ（同期が必要な場合）

2. **エラーハンドリング:**
   - 常にストレージ操作のエラーをキャッチする
   - 制限超過時の適切な処理を実装する

3. **パフォーマンス:**
   - 頻繁な書き込みを避ける（バッチ処理を検討）
   - 必要なキーのみを取得する（`get(null)` を多用しない）
   - ストレージ変更リスナーを適切に使用する

4. **Web Storage APIとの違い:**
   - ✗ Service Workerで `localStorage` は使用不可
   - ✗ `localStorage` は履歴削除で消える
   - ✅ `chrome.storage.local` は永続的

### 3.6 ストレージ変更の監視

```javascript
// ストレージ変更をリスナーで監視
chrome.storage.onChanged.addListener((changes, areaName) => {
  if (areaName === 'local') {
    if (changes.syncQueue) {
      console.log('Sync queue updated');
      const newQueue = changes.syncQueue.newValue || [];
      console.log(`Queue length: ${newQueue.length}`);
    }

    if (changes.accessToken) {
      console.log('Access token updated');
    }
  }
});
```

### 3.7 manifest.json の設定

**unlimitedStorageパーミッションの追加（オプション）:**

```json
{
  "manifest_version": 3,
  "permissions": [
    "storage",
    "unlimitedStorage"  // オプション: 10MBを超える保存が必要な場合
  ]
}
```

---

## 4. エラーハンドリングパターン

### 4.1 Google API エラーコード一覧

| HTTPコード | 意味 | 主な原因 | 対応方法 | リトライ |
|-----------|-----|---------|---------|---------|
| **400** | Bad Request | パラメータ不足、無効な値、重複する親 | リクエストパラメータを確認、入力を検証 | ✗ |
| **401** | Unauthorized | アクセストークンの期限切れ・無効 | リフレッシュトークンで新しいアクセストークンを取得 | △ (再認証後) |
| **403** | Forbidden | 権限不足、レート制限、ストレージクォータ超過 | 権限確認、exponential backoffで再試行 | ✓ (レート制限時) |
| **404** | Not Found | ドキュメントが存在しない、アクセス不可 | ファイルの存在確認、アクセス権の確認 | ✗ |
| **429** | Too Many Requests | APIリクエストレート超過 | Exponential backoffで再試行 | ✓ |
| **500** | Internal Server Error | サーバー側の予期しないエラー | Exponential backoffで再試行 | ✓ |
| **502** | Bad Gateway | バックエンドエラー | Exponential backoffで再試行 | ✓ |
| **503** | Service Unavailable | サービス一時的に利用不可 | Exponential backoffで再試行 | ✓ |
| **504** | Gateway Timeout | ゲートウェイタイムアウト | Exponential backoffで再試行 | ✓ |

**リトライ可能なエラーコード:** 408, 429, 500, 502, 503, 504

### 4.2 Exponential Backoff アルゴリズム

#### 4.2.1 アルゴリズムの概要

Exponential Backoffは、失敗したリクエストを指数関数的に増加する待機時間で再試行するアルゴリズムです。

**待機時間の計算式:**
```
waitTime = min((2^n + jitter), maxBackoff)
```

- `n`: リトライ回数（0から開始）
- `jitter`: ランダムな遅延（0～1000ミリ秒）
- `maxBackoff`: 最大待機時間（通常30～60秒）

**パラメータの推奨値:**
- 初回待機時間: 1秒
- 乗数: 2 または 3
- 最大待機時間: 30～60秒
- 最大リトライ回数: 3～7回
- 合計リトライ時間: 120～500秒

#### 4.2.2 JavaScript実装例

**基本的な実装:**

```javascript
/**
 * Exponential backoffでリトライを行う汎用関数
 * @param {Function} fn - リトライする非同期関数
 * @param {Object} options - オプション設定
 * @returns {Promise} - 関数の実行結果
 */
async function callWithRetry(fn, options = {}) {
  const {
    maxRetries = 7,        // 最大リトライ回数
    baseDelay = 1000,      // 初回待機時間（ミリ秒）
    factor = 2,            // 乗数
    maxDelay = 60000,      // 最大待機時間（ミリ秒）
    retryableErrors = [408, 429, 500, 502, 503, 504]  // リトライ可能なエラーコード
  } = options;

  async function attempt(retryCount = 0) {
    try {
      return await fn();
    } catch (error) {
      // リトライ不可能なエラーの場合は即座にスロー
      if (!shouldRetry(error, retryableErrors)) {
        throw error;
      }

      // 最大リトライ回数を超えた場合
      if (retryCount >= maxRetries) {
        console.error(`Max retries (${maxRetries}) exceeded`);
        throw error;
      }

      // 待機時間を計算（jitterを含む）
      const exponentialDelay = Math.min(
        baseDelay * Math.pow(factor, retryCount),
        maxDelay
      );
      const jitter = Math.random() * 1000; // 0～1000msのランダムな遅延
      const delay = exponentialDelay + jitter;

      console.log(`Retry ${retryCount + 1}/${maxRetries} after ${delay.toFixed(0)}ms`);

      // 待機
      await wait(delay);

      // 再試行
      return attempt(retryCount + 1);
    }
  }

  return attempt(0);
}

/**
 * エラーがリトライ可能かどうかを判定
 */
function shouldRetry(error, retryableErrors) {
  // HTTPステータスコードのチェック
  if (error.status && retryableErrors.includes(error.status)) {
    return true;
  }

  // ネットワークエラーのチェック
  if (error.message && error.message.includes('network')) {
    return true;
  }

  return false;
}

/**
 * 指定時間待機する
 */
function wait(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}
```

**使用例:**

```javascript
// Google Docs APIへのリクエストをリトライ付きで実行
const result = await callWithRetry(async () => {
  const accessToken = await getValidAccessToken();

  const response = await fetch(
    `https://docs.googleapis.com/v1/documents/${documentId}:batchUpdate`,
    {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${accessToken}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ requests })
    }
  );

  if (!response.ok) {
    const error = new Error(`API request failed: ${response.status}`);
    error.status = response.status;
    throw error;
  }

  return await response.json();
}, {
  maxRetries: 5,
  baseDelay: 1000,
  factor: 2,
  maxDelay: 30000
});
```

#### 4.2.3 より洗練された実装（Google Cloud推奨）

```javascript
/**
 * Google Cloud Storage推奨のExponential Backoff実装
 */
class RetryWithBackoff {
  constructor(options = {}) {
    this.initialDelay = options.initialDelay || 1000;      // 1秒
    this.multiplier = options.multiplier || 2;             // 2倍
    this.maxDelay = options.maxDelay || 60000;             // 60秒
    this.maxRetries = options.maxRetries || 7;
    this.timeout = options.timeout || 300000;              // 5分
  }

  async execute(fn) {
    const startTime = Date.now();
    let currentDelay = this.initialDelay;
    let retryCount = 0;

    while (true) {
      try {
        return await fn();
      } catch (error) {
        retryCount++;

        // タイムアウトチェック
        const elapsed = Date.now() - startTime;
        if (elapsed >= this.timeout) {
          throw new Error(`Retry timeout exceeded (${this.timeout}ms)`);
        }

        // 最大リトライ回数チェック
        if (retryCount > this.maxRetries) {
          throw new Error(`Max retries exceeded (${this.maxRetries})`);
        }

        // リトライ不可能なエラー
        if (!this.isRetryable(error)) {
          throw error;
        }

        // Jitterを追加
        const jitter = Math.random() * currentDelay * 0.1;
        const delayWithJitter = currentDelay + jitter;

        console.log(
          `Retry ${retryCount}/${this.maxRetries} ` +
          `after ${delayWithJitter.toFixed(0)}ms ` +
          `(error: ${error.status || error.message})`
        );

        await this.wait(delayWithJitter);

        // 次回の待機時間を計算
        currentDelay = Math.min(currentDelay * this.multiplier, this.maxDelay);
      }
    }
  }

  isRetryable(error) {
    // HTTPステータスコードによる判定
    const retryableStatuses = [408, 429, 500, 502, 503, 504];
    if (error.status && retryableStatuses.includes(error.status)) {
      return true;
    }

    // ネットワークエラー
    if (error.message &&
        (error.message.includes('network') ||
         error.message.includes('timeout') ||
         error.message.includes('ECONNRESET'))) {
      return true;
    }

    return false;
  }

  wait(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

// 使用例
const retry = new RetryWithBackoff({
  initialDelay: 1500,
  multiplier: 1.5,
  maxDelay: 45000,
  maxRetries: 5,
  timeout: 300000
});

const result = await retry.execute(async () => {
  return await callGoogleDocsAPI(documentId, requests);
});
```

### 4.3 エラー固有の処理

#### 4.3.1 401 Unauthorized

アクセストークンが無効または期限切れの場合。

```javascript
async function handleUnauthorizedError() {
  try {
    // リフレッシュトークンで新しいアクセストークンを取得
    const items = await chrome.storage.local.get(['refreshToken']);

    if (items.refreshToken) {
      const newTokens = await refreshAccessToken(items.refreshToken);
      console.log('Access token refreshed');
      return true;
    } else {
      // リフレッシュトークンがない場合は再認証が必要
      console.log('No refresh token, re-authentication required');
      await chrome.storage.local.clear();
      return false;
    }
  } catch (error) {
    // リフレッシュトークンも無効な場合
    console.error('Refresh token invalid, re-authentication required');
    await chrome.storage.local.clear();
    return false;
  }
}
```

#### 4.3.2 403 Forbidden（権限エラー）

```javascript
async function handleForbiddenError(error) {
  // レート制限の場合はリトライ
  if (error.message && error.message.includes('rate limit')) {
    console.log('Rate limit exceeded, retrying with backoff...');
    return 'retry';
  }

  // 権限不足の場合
  if (error.message && error.message.includes('permission')) {
    console.error('Insufficient permissions for this operation');
    // ユーザーに通知
    return 'permission_denied';
  }

  // ストレージクォータ超過
  if (error.message && error.message.includes('storage')) {
    console.error('Storage quota exceeded');
    return 'quota_exceeded';
  }

  return 'unknown_forbidden';
}
```

#### 4.3.3 404 Not Found

```javascript
async function handleNotFoundError(documentId) {
  console.error(`Document not found: ${documentId}`);

  // ユーザーに通知
  await showNotification({
    title: 'Document not found',
    message: 'The Google Docs document may have been deleted or you may not have access.',
    type: 'error'
  });

  // キューから削除
  const { syncQueue = [] } = await chrome.storage.local.get(['syncQueue']);
  const updatedQueue = syncQueue.filter(item => item.documentId !== documentId);
  await chrome.storage.local.set({ syncQueue: updatedQueue });
}
```

### 4.4 オフラインエラーハンドリング

#### 4.4.1 ネットワーク接続の検出

```javascript
// ネットワーク状態の監視
let isOnline = navigator.onLine;

window.addEventListener('online', () => {
  console.log('Network connection restored');
  isOnline = true;
  // キューの処理を開始
  processSyncQueue();
});

window.addEventListener('offline', () => {
  console.log('Network connection lost');
  isOnline = false;
});

// ネットワーク接続のチェック
async function checkNetworkConnection() {
  if (!navigator.onLine) {
    return false;
  }

  try {
    // 実際にリクエストを送信して確認
    const response = await fetch('https://www.google.com/favicon.ico', {
      method: 'HEAD',
      cache: 'no-cache'
    });
    return response.ok;
  } catch (error) {
    return false;
  }
}
```

#### 4.4.2 オフライン時のキューイング

```javascript
async function syncToGoogleDocs(documentId, requests) {
  // ネットワーク接続をチェック
  const isConnected = await checkNetworkConnection();

  if (!isConnected) {
    console.log('Offline: Adding to sync queue');
    await addToSyncQueue({
      documentId,
      operation: 'append',
      requests
    });

    // ユーザーに通知
    await showNotification({
      title: 'Saved for later',
      message: 'Your changes will be synced when you\'re back online.',
      type: 'info'
    });

    return { queued: true };
  }

  // オンラインの場合は即座に同期
  try {
    const result = await callWithRetry(async () => {
      return await callGoogleDocsAPI(documentId, requests);
    });

    return { success: true, result };
  } catch (error) {
    // 同期失敗時はキューに追加
    console.error('Sync failed, adding to queue:', error);
    await addToSyncQueue({
      documentId,
      operation: 'append',
      requests
    });

    return { queued: true, error };
  }
}
```

### 4.5 Workbox Background Sync（オプション）

Chrome拡張機能のService Workerでより高度なオフライン同期を実装する場合、Workboxを使用できます。

#### 4.5.1 Workbox Queueの使用

```javascript
import { Queue } from 'workbox-background-sync';

// キューを作成
const syncQueue = new Queue('google-docs-sync-queue', {
  maxRetentionTime: 24 * 60  // 24時間
});

// フェッチイベントでキューイング
self.addEventListener('fetch', (event) => {
  if (event.request.url.includes('docs.googleapis.com') &&
      event.request.method === 'POST') {

    const syncLogic = async () => {
      try {
        const response = await fetch(event.request.clone());
        return response;
      } catch (error) {
        // 失敗した場合はキューに追加
        await syncQueue.pushRequest({ request: event.request });
        return new Response('Queued for sync', { status: 202 });
      }
    };

    event.respondWith(syncLogic());
  }
});
```

#### 4.5.2 Background Syncイベント

```javascript
// Service Workerでbackground syncイベントをリスン
self.addEventListener('sync', (event) => {
  if (event.tag === 'google-docs-sync') {
    event.waitUntil(processSyncQueue());
  }
});

// キュー処理の登録
async function registerBackgroundSync() {
  const registration = await navigator.serviceWorker.ready;
  await registration.sync.register('google-docs-sync');
}
```

**注意事項:**
- Background Sync APIはChrome拡張機能のService Workerで利用可能
- 接続回復時に自動的にsyncイベントが発火する
- Chromeは3回まで自動リトライを試みる（初回、5分後、15分後）

### 4.6 エラーログとモニタリング

#### 4.6.1 構造化されたエラーログ

```javascript
class ErrorLogger {
  static log(error, context = {}) {
    const errorLog = {
      timestamp: new Date().toISOString(),
      message: error.message,
      stack: error.stack,
      status: error.status,
      context: {
        ...context,
        userAgent: navigator.userAgent,
        online: navigator.onLine
      }
    };

    console.error('Error logged:', errorLog);

    // ローカルストレージに保存（デバッグ用）
    this.saveToStorage(errorLog);

    // 将来的にはエラートラッキングサービスに送信
    // this.sendToErrorTracking(errorLog);
  }

  static async saveToStorage(errorLog) {
    const { errorLogs = [] } = await chrome.storage.local.get(['errorLogs']);

    // 最新100件のみ保持
    errorLogs.push(errorLog);
    if (errorLogs.length > 100) {
      errorLogs.shift();
    }

    await chrome.storage.local.set({ errorLogs });
  }

  static async getRecentErrors(limit = 10) {
    const { errorLogs = [] } = await chrome.storage.local.get(['errorLogs']);
    return errorLogs.slice(-limit);
  }
}

// 使用例
try {
  await callGoogleDocsAPI(documentId, requests);
} catch (error) {
  ErrorLogger.log(error, {
    operation: 'batchUpdate',
    documentId: documentId,
    requestCount: requests.length
  });
  throw error;
}
```

---

## 5. 実装推奨事項

### 5.1 アーキテクチャ推奨事項

#### 5.1.1 レイヤー分離

```
┌─────────────────────────────────────┐
│     UI Layer (Popup/Options)        │
│  - ユーザーインタラクション           │
│  - 設定画面                          │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│   Application Layer (Background)    │
│  - 認証管理 (AuthManager)            │
│  - 同期管理 (SyncManager)            │
│  - キュー管理 (QueueManager)         │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│    API Layer (Google Docs Client)   │
│  - batchUpdate実装                   │
│  - Named Range管理                   │
│  - エラーハンドリング                 │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│   Infrastructure Layer              │
│  - Exponential Backoff               │
│  - ネットワーク監視                   │
│  - ストレージ管理                     │
└─────────────────────────────────────┘
```

#### 5.1.2 主要クラスの責務

**AuthManager:**
```javascript
class AuthManager {
  async authenticate() { /* OAuth 2.0フロー */ }
  async getValidAccessToken() { /* トークン取得・リフレッシュ */ }
  async refreshAccessToken(refreshToken) { /* トークンリフレッシュ */ }
  async signOut() { /* トークン削除・取り消し */ }
  isAuthenticated() { /* 認証状態確認 */ }
}
```

**GoogleDocsClient:**
```javascript
class GoogleDocsClient {
  async batchUpdate(documentId, requests) { /* batchUpdate実装 */ }
  async createNamedRange(documentId, name, range) { /* Named Range作成 */ }
  async replaceNamedRangeContent(documentId, rangeName, text) { /* コンテンツ置換 */ }
  async getDocument(documentId) { /* ドキュメント取得 */ }
}
```

**SyncManager:**
```javascript
class SyncManager {
  async syncToGoogleDocs(documentId, content) { /* 同期実行 */ }
  async addToQueue(item) { /* キューに追加 */ }
  async processQueue() { /* キュー処理 */ }
  async onNetworkRestored() { /* ネットワーク復旧時の処理 */ }
}
```

**QueueManager:**
```javascript
class QueueManager {
  async add(item) { /* キューアイテム追加 */ }
  async getAll() { /* 全キューアイテム取得 */ }
  async remove(id) { /* キューアイテム削除 */ }
  async clear() { /* キュークリア */ }
  async updateRetryCount(id, count) { /* リトライ回数更新 */ }
}
```

### 5.2 セキュリティ推奨事項

1. **最小権限の原則:**
   - `drive.file` スコープを使用（`drive` や `documents` スコープを避ける）
   - 必要最小限のパーミッションのみをmanifest.jsonに記載

2. **トークン管理:**
   - トークンは`chrome.storage.local`に保存（暗号化されていないことを認識）
   - 不要になったトークンは即座に削除
   - トークンをログに出力しない

3. **HTTPS通信:**
   - すべてのAPI通信はHTTPSを使用
   - `host_permissions`で明示的にエンドポイントを指定

4. **CSP (Content Security Policy):**
   ```json
   {
     "content_security_policy": {
       "extension_pages": "script-src 'self'; object-src 'self'"
     }
   }
   ```

### 5.3 パフォーマンス推奨事項

1. **バッチ処理:**
   - 複数の変更を1回のbatchUpdateリクエストにまとめる
   - APIコール数を最小限に抑える

2. **キャッシング:**
   - ドキュメントメタデータをキャッシュ
   - Named Range情報をキャッシュ（有効期限を設定）

3. **ストレージ最適化:**
   - 不要な古いキューアイテムを定期的に削除
   - ストレージ使用量を監視

4. **レート制限の考慮:**
   - 書き込みリクエスト: 60/分/ユーザー
   - 必要に応じてリクエストをスロットリング

### 5.4 ユーザーエクスペリエンス推奨事項

1. **フィードバック:**
   - 同期状態をユーザーに明示的に表示
   - オフライン時の動作を説明
   - エラー時のわかりやすいメッセージ

2. **通知:**
   ```javascript
   chrome.notifications.create({
     type: 'basic',
     iconUrl: 'icons/icon48.png',
     title: 'Synced to Google Docs',
     message: 'Meeting minutes have been updated successfully.'
   });
   ```

3. **プログレス表示:**
   - キュー処理中のプログレス表示
   - 同期中のローディング状態

### 5.5 テスト推奨事項

1. **ユニットテスト:**
   - AuthManager: トークンリフレッシュロジック
   - ExponentialBackoff: リトライロジック
   - QueueManager: キュー管理ロジック

2. **統合テスト:**
   - Google Docs APIとの実際の通信
   - オフライン→オンライン遷移時の動作

3. **エッジケース:**
   - ネットワーク切断中の動作
   - トークン期限切れ時の動作
   - レート制限到達時の動作
   - ストレージ容量超過時の動作

### 5.6 コード例: 完全な実装

**background.js（Service Worker）:**

```javascript
import { AuthManager } from './auth/AuthManager.js';
import { GoogleDocsClient } from './api/GoogleDocsClient.js';
import { SyncManager } from './sync/SyncManager.js';
import { QueueManager } from './sync/QueueManager.js';

// インスタンスの初期化
const authManager = new AuthManager();
const queueManager = new QueueManager();
const docsClient = new GoogleDocsClient(authManager);
const syncManager = new SyncManager(docsClient, queueManager);

// ネットワーク状態の監視
chrome.alarms.create('checkNetworkAndSync', { periodInMinutes: 5 });

chrome.alarms.onAlarm.addListener(async (alarm) => {
  if (alarm.name === 'checkNetworkAndSync') {
    if (navigator.onLine) {
      await syncManager.processQueue();
    }
  }
});

// メッセージハンドラ
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  (async () => {
    try {
      switch (request.type) {
        case 'authenticate':
          const authResult = await authManager.authenticate();
          sendResponse({ success: true, data: authResult });
          break;

        case 'syncToGoogleDocs':
          const syncResult = await syncManager.syncToGoogleDocs(
            request.documentId,
            request.content
          );
          sendResponse({ success: true, data: syncResult });
          break;

        case 'getQueueStatus':
          const queue = await queueManager.getAll();
          sendResponse({ success: true, data: { queueLength: queue.length } });
          break;

        case 'signOut':
          await authManager.signOut();
          sendResponse({ success: true });
          break;

        default:
          sendResponse({ success: false, error: 'Unknown message type' });
      }
    } catch (error) {
      console.error('Error handling message:', error);
      sendResponse({ success: false, error: error.message });
    }
  })();

  return true; // 非同期レスポンスのために必要
});
```

---

## 6. 重要な警告と制限事項

### 6.1 Google Docs API

⚠️ **Named Rangeの可視性:**
- Named Rangeはプライベートではなく、ドキュメントへのアクセス権を持つ全員が閲覧可能
- 機密情報をNamed Range名に含めない

⚠️ **コラボレーション:**
- 他のユーザーが同時に編集している場合、インデックスがずれる可能性がある
- `writeControl.requiredRevisionId`を使用して競合を検出可能

⚠️ **レート制限:**
- 書き込みリクエストは60/分/ユーザーと厳しい制限がある
- 大量の同期が必要な場合はバッチ処理とスロットリングが必須

### 6.2 Chrome Storage API

⚠️ **暗号化なし:**
- `chrome.storage.local`は暗号化されていない
- アクセストークンやリフレッシュトークンが平文で保存される
- より強固なセキュリティが必要な場合はサーバー側での管理を検討

⚠️ **ストレージ制限:**
- デフォルト10MB（unlimitedStorageパーミッションで無制限）
- 大量の議事録をキューイングする場合は制限に注意

### 6.3 OAuth 2.0

⚠️ **Client Secretの扱い:**
- Chrome拡張機能ではClient Secretを完全に秘密に保つことはできない
- Chromeウェブストアで配布する場合、コードは公開されているものとして扱う
- 可能であればPKCE (Proof Key for Code Exchange) フローを使用

⚠️ **Refresh Tokenの取得:**
- `access_type: 'offline'` と `prompt: 'consent'` を指定する必要がある
- 既に同意済みのユーザーには再度同意画面が表示される

### 6.4 Background Sync

⚠️ **ブラウザサポート:**
- Background Sync APIはChrome/Edge/Operaでサポート
- Firefoxでは未サポート（フォールバック実装が必要）

⚠️ **Service Workerのライフサイクル:**
- Chrome拡張機能のService Workerは5分でタイムアウト
- 長時間のタスクはAlarmsAPIと組み合わせる

---

## 7. 参考リンク

### 公式ドキュメント

**Google Docs API:**
- [documents.batchUpdate リファレンス](https://developers.google.com/docs/api/reference/rest/v1/documents/batchUpdate)
- [Named Rangesガイド](https://developers.google.com/workspace/docs/api/how-tos/named-ranges)
- [認証とスコープ](https://developers.google.com/workspace/docs/api/auth)
- [使用制限とクォータ](https://developers.google.com/docs/api/limits)
- [エラーハンドリング](https://developers.google.com/workspace/drive/api/guides/handle-errors)

**Chrome Extensions:**
- [chrome.identity API](https://developer.chrome.com/docs/extensions/reference/api/identity)
- [chrome.storage API](https://developer.chrome.com/docs/extensions/reference/api/storage)
- [OAuth 2.0 for Chrome Extensions](https://developer.chrome.com/docs/extensions/how-to/integrate/oauth)

**OAuth 2.0:**
- [OAuth 2.0 for Google APIs](https://developers.google.com/identity/protocols/oauth2)
- [OAuth 2.0 Scopes](https://developers.google.com/identity/protocols/oauth2/scopes)

**Background Sync:**
- [Workbox Background Sync](https://developer.chrome.com/docs/workbox/modules/workbox-background-sync)
- [Background Sync API](https://developer.chrome.com/blog/background-sync)

**エラーハンドリング:**
- [Google Cloud Retry Strategy](https://cloud.google.com/storage/docs/retry-strategy)
- [Exponential Backoff実装ガイド](https://advancedweb.hu/how-to-implement-an-exponential-backoff-retry-strategy-in-javascript/)

---

## 8. まとめ

### 8.1 主要な技術的決定事項

1. **認証:**
   - `chrome.identity.launchWebAuthFlow()` を使用
   - `drive.file` スコープを使用（Non-sensitive）
   - アクセストークン + リフレッシュトークンを`chrome.storage.local`に保存

2. **Google Docs API:**
   - `documents.batchUpdate` で複数の操作を原子的に実行
   - Named Rangeで議事録セクションを管理
   - レート制限: 60書き込み/分/ユーザーを考慮

3. **オフライン対応:**
   - `chrome.storage.local`でキュー管理
   - Exponential Backoffでリトライ
   - ネットワーク復旧時に自動同期

4. **エラーハンドリング:**
   - 401: トークンリフレッシュ
   - 403/429: Exponential Backoffでリトライ
   - 500-504: Exponential Backoffでリトライ
   - 400/404: リトライせず、ユーザーに通知

### 8.2 実装時の注意点

✅ **推奨事項:**
- 最小権限のスコープを使用（`drive.file`）
- トークンは60秒のバッファを持ってリフレッシュ
- Named Rangeでセクション管理を簡素化
- Exponential Backoffでレート制限に対応
- オフライン時はキューイングして後で同期

⚠️ **注意事項:**
- Chrome Storage APIは暗号化されていない
- Named Rangeは公開情報
- レート制限が厳しい（60書き込み/分/ユーザー）
- Service Workerは5分でタイムアウト

### 8.3 次のステップ

1. **設計フェーズ:**
   - クラス構造の詳細設計
   - データフローの設計
   - エラーハンドリング戦略の詳細化

2. **実装フェーズ:**
   - AuthManagerの実装
   - GoogleDocsClientの実装
   - SyncManagerとQueueManagerの実装
   - Exponential Backoffユーティリティの実装

3. **テストフェーズ:**
   - ユニットテスト
   - 統合テスト（実際のGoogle Docs APIとの通信）
   - オフライン/オンライン遷移テスト
   - レート制限到達テスト

---

**調査完了日**: 2025-10-03
**調査者**: Claude Code (Sonnet 4.5)
