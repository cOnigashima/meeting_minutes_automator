# Technical Design - meeting-minutes-docs-sync: Components and Interfaces

> **プロジェクト**: OAuth 2.0 + Google Docs API統合でGoogle Meetから議事録を自動同期（MVP2）
> **親ドキュメント**: [design.md](../design.md)
> **関連**: [Requirements](../requirements.md) | [Tasks](../tasks.md) | [他のモジュール](README.md)

## Components and Interfaces

### Auth Domain

#### AuthManager

**Responsibility & Boundaries**
- **Primary Responsibility**: OAuth 2.0認証フローの実行とトークンライフサイクル管理
- **Domain Boundary**: Chrome Extension内の認証レイヤー（他のドメインからはインターフェースを通じてアクセス）
- **Data Ownership**: アクセストークン、リフレッシュトークン、有効期限の管理
- **Transaction Boundary**: トークン取得/リフレッシュ/無効化の一貫性を保証

**Dependencies**
- **Inbound**: SyncManager、GoogleDocsClient（トークンを要求）
- **Outbound**: Chrome Identity API、TokenStore、Google OAuth 2.0エンドポイント
- **External**: `chrome.identity.launchWebAuthFlow()`、Google OAuth 2.0 Token Endpoint

**Contract Definition**

**Service Interface**:
```typescript
interface AuthManager {
  /**
   * OAuth 2.0認証フローを開始し、アクセストークンとリフレッシュトークンを取得する
   *
   * @preconditions chrome.identity APIが利用可能
   * @postconditions トークンがTokenStoreに保存される
   * @invariants リフレッシュトークンは有効期限なし（無効化されるまで有効）
   * @throws AuthError トークン取得失敗、ユーザーキャンセル
   */
  initiateAuth(): Promise<Result<AuthTokens, AuthError>>;

  /**
   * アクセストークンを取得する。期限切れの場合は自動リフレッシュ
   *
   * @preconditions TokenStoreにリフレッシュトークンが保存されている
   * @postconditions 有効なアクセストークンが返される
   * @invariants 期限切れ60秒前に自動リフレッシュ
   * @throws TokenExpiredError リフレッシュトークンも無効
   */
  getAccessToken(): Promise<Result<string, TokenExpiredError>>;

  /**
   * リフレッシュトークンを使用してアクセストークンを更新する
   *
   * @preconditions リフレッシュトークンが有効
   * @postconditions 新しいアクセストークンがTokenStoreに保存される
   * @throws RefreshError リフレッシュトークンが無効
   */
  refreshToken(): Promise<Result<string, RefreshError>>;

  /**
   * トークンを無効化し、TokenStoreから削除する
   *
   * @postconditions TokenStoreが空になる
   * @throws RevokeError トークン無効化失敗（ベストエフォート）
   */
  revokeToken(): Promise<Result<void, RevokeError>>;
}

type AuthTokens = {
  accessToken: string;
  refreshToken: string;
  expiresAt: number; // Unix timestamp
};

type AuthError =
  | { type: 'UserCancelled' }
  | { type: 'NetworkError'; message: string }
  | { type: 'InvalidGrant'; message: string };

type TokenExpiredError = {
  type: 'RefreshRequired';
  message: string;
};

type RefreshError = {
  type: 'InvalidRefreshToken';
  message: string;
};

type RevokeError = {
  type: 'RevokeFailed';
  message: string;
};
```

**State Management**:
- **State Model**: `NotAuthenticated` → `Authenticating` → `Authenticated` → `TokenExpired` → `Authenticated` (ループ)
- **Persistence**: `chrome.storage.local` (`auth_tokens` key)
- **Concurrency**: トークンリフレッシュ中は他のAPIリクエストを待機（Mutex）

**Integration Strategy**:
- **Modification Approach**: 新規追加（Chrome拡張に認証レイヤーを追加）
- **Backward Compatibility**: 既存のWebSocket通信には影響なし

---

#### TokenStore

**Responsibility & Boundaries**
- **Primary Responsibility**: OAuth 2.0トークンの永続化とCRUD操作
- **Domain Boundary**: Auth Domainのデータ永続化層
- **Data Ownership**: アクセストークン、リフレッシュトークン、有効期限
- **Transaction Boundary**: 単一トークンセットの読み書き（原子性保証）

**Dependencies**
- **Inbound**: AuthManager
- **Outbound**: `chrome.storage.local`

**Contract Definition**:
```typescript
interface TokenStore {
  /**
   * トークンを保存する
   *
   * @preconditions chrome.storage.localが利用可能
   * @postconditions トークンがストレージに永続化される
   * @throws StorageError ストレージ書き込み失敗
   */
  saveTokens(tokens: AuthTokens): Promise<Result<void, StorageError>>;

  /**
   * トークンを取得する
   *
   * @postconditions トークンが存在しない場合はnullを返す
   */
  getTokens(): Promise<AuthTokens | null>;

  /**
   * トークンを削除する
   *
   * @postconditions ストレージからトークンが削除される
   */
  clearTokens(): Promise<void>;
}

type StorageError = {
  type: 'QuotaExceeded' | 'WriteError';
  message: string;
};
```

---

### Sync Domain

#### SyncManager

**Responsibility & Boundaries**
- **Primary Responsibility**: 文字起こしメッセージの受信、オンライン/オフライン状態管理、自動同期制御
- **Domain Boundary**: Sync Domainの中心コンポーネント（オーケストレーター）
- **Data Ownership**: 同期ステータス、ドキュメントID、現在の同期モード
- **Transaction Boundary**: 単一メッセージの処理（オンライン/オフラインへのルーティング）

**Dependencies**
- **Inbound**: Background Worker（WebSocketメッセージ受信）
- **Outbound**: QueueManager、GoogleDocsClient、Tauri App（WebSocket）
- **External**: ネットワーク状態監視（`navigator.onLine`）

**Contract Definition**:
```typescript
interface SyncManager {
  /**
   * Google Docs同期を開始する
   *
   * @preconditions AuthManagerで認証済み、Google Docsタブがアクティブ
   * @postconditions ドキュメントIDが取得され、Named Rangeが作成される
   * @throws SyncStartError ドキュメントID取得失敗、Named Range作成失敗
   */
  startSync(documentId: string): Promise<Result<void, SyncStartError>>;

  /**
   * 文字起こしメッセージを処理する
   *
   * @preconditions startSync()実行済み
   * @postconditions オンライン時はGoogleDocsClientへ送信、オフライン時はQueueManagerへ保存
   */
  processTranscription(message: TranscriptionMessage): Promise<Result<void, ProcessError>>;

  /**
   * ネットワーク復帰時にオフラインキューを再同期する
   *
   * @preconditions オフラインキューにメッセージが存在
   * @postconditions 全メッセージの送信完了後にキューがクリアされる
   * @throws ResyncError 再送信中のエラー
   */
  resyncOfflineQueue(): Promise<Result<void, ResyncError>>;

  /**
   * Google Docs同期を停止する
   *
   * @postconditions 同期ステータスがリセットされる
   */
  stopSync(): Promise<void>;

  /**
   * 現在の同期ステータスを取得する
   */
  getStatus(): SyncStatus;
}

type SyncStatus = {
  mode: 'online' | 'offline' | 'stopped';
  documentId: string | null;
  documentTitle: string | null;
  queuedMessages: number;
};

type TranscriptionMessage = {
  messageId: number;
  sessionId: string;
  timestamp: number;
  type: 'transcription';
  isPartial: boolean;
  text: string;
  confidence?: number;
  language?: string;
};

type SyncStartError =
  | { type: 'NotAuthenticated' }
  | { type: 'InvalidDocumentId' }
  | { type: 'NamedRangeCreationFailed'; message: string };

type ProcessError = {
  type: 'SyncNotStarted' | 'NetworkError' | 'QueueFull';
  message: string;
};

type ResyncError = {
  type: 'PartialFailure';
  failedMessages: TranscriptionMessage[];
  message: string;
};
```

**Event Contract** (WebSocket経由でTauriアプリへ送信):
```typescript
type SyncEvent =
  | { type: 'docs_sync_started'; documentId: string; documentTitle: string; timestamp: number }
  | { type: 'docs_sync_offline'; queuedMessages: number; timestamp: number }
  | { type: 'docs_sync_online'; resyncInProgress: boolean; timestamp: number }
  | { type: 'docs_sync_success'; messageId: number; insertedAt: string }
  | { type: 'docs_sync_error'; messageId: number; error: string };
```

**State Management**:
- **State Model**: `Stopped` → `Starting` → `OnlineSync` ⇄ `OfflineQueue` → `Resyncing` → `OnlineSync`
- **Persistence**: `chrome.storage.local` (`sync_status` key)
- **Concurrency**: 再同期中は新規メッセージを一時バッファに保存

---

#### QueueManager

**Responsibility & Boundaries**
- **Primary Responsibility**: オフラインキューの管理（追加、取得、クリア、ストレージ監視）
- **Domain Boundary**: Sync Domainのデータ永続化層
- **Data Ownership**: オフラインキューのメッセージリスト、ストレージ使用量
- **Transaction Boundary**: 単一メッセージの追加/削除操作（原子性保証）

**Dependencies**
- **Inbound**: SyncManager
- **Outbound**: `chrome.storage.local`、`chrome.notifications`
- **External**: ストレージAPI

**Contract Definition**:
```typescript
interface QueueManager {
  /**
   * メッセージをオフラインキューに追加する
   *
   * @preconditions ストレージ使用量が上限未満
   * @postconditions メッセージがキューに追加される
   * @throws QueueFullError ストレージ上限到達
   */
  enqueue(message: TranscriptionMessage): Promise<Result<void, QueueFullError>>;

  /**
   * オフラインキューの全メッセージを取得する（時系列順）
   *
   * @postconditions メッセージがtimestamp昇順でソートされる
   */
  getAll(): Promise<TranscriptionMessage[]>;

  /**
   * オフラインキューをクリアする
   *
   * @postconditions キューが空になる
   */
  clear(): Promise<void>;

  /**
   * ストレージ使用量を取得する
   *
   * @returns 使用量（0.0～1.0）
   */
  getStorageUsage(): Promise<number>;

  /**
   * ストレージ使用量を監視し、警告を表示する
   *
   * @postconditions 80%超でポップアップ警告、100%で全画面通知
   */
  monitorStorage(): void;
}

type QueueFullError = {
  type: 'StorageLimitReached';
  currentSize: number;
  maxSize: number;
};
```

**Monitoring Strategy**:
```typescript
// DOCS-REQ-005.11: 80%到達時の警告
if (storageUsage >= 0.8) {
  showPopupWarning(`オフラインキューが残り${Math.floor((1 - storageUsage) * MAX_QUEUE_SIZE)}件です`);
}

// DOCS-REQ-005.12: 100%到達時の全画面通知
if (storageUsage >= 1.0) {
  chrome.notifications.create({
    type: 'basic',
    iconUrl: 'icon.png',
    title: 'オフラインキュー上限到達',
    message: 'これ以上の文字起こしは保存されません。録音を停止するか、ネットワーク接続を回復してください',
    priority: 2
  });
}
```

---

### API Domain

#### GoogleDocsClient

**Responsibility & Boundaries**
- **Primary Responsibility**: Google Docs API呼び出しの抽象化、レート制限対応、エラーハンドリング
- **Domain Boundary**: API Domainの外部通信層
- **Data Ownership**: APIリクエスト/レスポンスの一時データ
- **Transaction Boundary**: 単一`batchUpdate`リクエストの原子性

**Dependencies**
- **Inbound**: SyncManager、NamedRangeManager
- **Outbound**: Google Docs API (`docs.googleapis.com`)
- **External**: Fetch API、AuthManager（トークン取得）

**External Dependencies Investigation**:

**Google Docs API v1 batchUpdate**:
- **公式ドキュメント**: https://developers.google.com/docs/api/reference/rest/v1/documents/batchUpdate
- **エンドポイント**: `POST https://docs.googleapis.com/v1/documents/{documentId}:batchUpdate`
- **認証**: `Authorization: Bearer {accessToken}` ヘッダー
- **レート制限**:
  - 書き込み: **60リクエスト/分/ユーザー**（重要）
  - 読み取り: 300リクエスト/分/ユーザー
  - プロジェクト全体: 600リクエスト/分（書き込み）
- **リクエストスキーマ**:
  ```json
  {
    "requests": [
      {
        "insertText": {
          "location": { "index": 1 },
          "text": "文字起こしテキスト\n"
        }
      }
    ],
    "writeControl": {
      "requiredRevisionId": "optional-revision-id"
    }
  }
  ```
- **レスポンススキーマ** (200 OK):
  ```json
  {
    "documentId": "string",
    "replies": [...],
    "writeControl": {
      "requiredRevisionId": "string"
    }
  }
  ```
- **エラーレスポンス**:
  - `400 Bad Request`: リクエスト形式エラー
  - `401 Unauthorized`: トークン無効
  - `403 Forbidden`: 権限不足
  - `404 Not Found`: ドキュメント不在
  - `429 Too Many Requests`: レート制限超過（Retry-After ヘッダー付き）
  - `500-504 Server Error`: Google側エラー

**Exponential Backoff実装例** (公式推奨):
```typescript
async function exponentialBackoff<T>(
  fn: () => Promise<T>,
  maxRetries: number = 5
): Promise<T> {
  let delay = 1000; // 初回1秒

  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      if (!isRetryableError(error) || i === maxRetries - 1) {
        throw error;
      }

      // Jitter（ランダム遅延）を追加
      const jitter = Math.random() * 1000;
      await sleep(delay + jitter);

      delay *= 2; // 指数バックオフ
      delay = Math.min(delay, 60000); // 最大60秒
    }
  }
}

function isRetryableError(error: any): boolean {
  const retryableCodes = [408, 429, 500, 502, 503, 504];
  return retryableCodes.includes(error.status);
}
```

**Contract Definition**:
```typescript
interface GoogleDocsClient {
  /**
   * Google Docsドキュメントを取得する
   *
   * @preconditions 有効なアクセストークンが存在
   * @postconditions ドキュメントの構造、Named Range、revisionIdを含むレスポンスが返される
   * @throws ApiError 401, 403, 404, 429, 500-504
   */
  getDocument(documentId: string): Promise<Result<Document, ApiError>>;

  /**
   * テキストを楽観ロック付きで挿入する（推奨）
   *
   * @preconditions Named Rangeで指定された位置が有効
   * @postconditions テキストが挿入され、Named Rangeが更新される。競合時は自動リトライ。
   * @throws ApiError 401, 403, 429, 500-504
   * @throws ConflictError 最大リトライ回数超過
   */
  insertTextWithLock(documentId: string, text: string, position: number): Promise<Result<void, ApiError | ConflictError>>;

  /**
   * テキストを挿入する（レガシー、楽観ロックなし）
   *
   * @deprecated insertTextWithLock() を使用してください
   * @preconditions Named Rangeで指定された位置が有効
   * @postconditions テキストが挿入され、Named Rangeが更新される
   * @throws ApiError 401, 403, 429, 500-504
   */
  insertText(documentId: string, text: string, position: number): Promise<Result<void, ApiError>>;

  /**
   * 複数のリクエストをバッチ実行する
   *
   * @preconditions リクエスト配列が空でない
   * @postconditions 全リクエストが原子的に実行される（全成功or全失敗）
   * @throws ApiError 401, 403, 429, 500-504
   */
  batchUpdate(documentId: string, requests: Request[], writeControl?: WriteControl): Promise<Result<BatchUpdateResponse, ApiError>>;
}

type Document = {
  documentId: string;
  title: string;
  revisionId: string; // 楽観ロック用のリビジョンID
  body: {
    content: ContentElement[];
  };
  namedRanges: Record<string, NamedRange>;
};

type NamedRange = {
  namedRangeId: string;
  name: string;
  ranges: { startIndex: number; endIndex: number }[];
};

type Request =
  | { insertText: { location: { index: number }; text: string } }
  | { createNamedRange: { name: string; range: { startIndex: number; endIndex: number } } }
  | { updateNamedRange: { namedRangeId: string; range: { startIndex: number; endIndex: number } } };

type WriteControl = {
  requiredRevisionId: string; // 楽観ロック用のリビジョンID
};

type BatchUpdateResponse = {
  documentId: string;
  replies: any[];
  writeControl: { requiredRevisionId: string };
};

type ApiError =
  | { type: 'Unauthorized'; status: 401; message: string; headers?: Record<string, string> }
  | { type: 'Forbidden'; status: 403; message: string; headers?: Record<string, string> }
  | { type: 'NotFound'; status: 404; message: string; headers?: Record<string, string> }
  | { type: 'RateLimitExceeded'; status: 429; retryAfter: number; headers?: Record<string, string> }
  | { type: 'ServerError'; status: 500 | 502 | 503 | 504; message: string; headers?: Record<string, string> };

type ConflictError = {
  type: 'RevisionMismatch';
  status: 400;
  message: string;
  retryCount: number;
};
```

**Optimistic Locking Implementation** (楽観ロック実装):

**重要**: 複数タブ/共同編集者との競合を防ぐため、`writeControl.requiredRevisionId`を使用した楽観ロックを実装する。

```typescript
class GoogleDocsClient {
  private readonly MAX_CONFLICT_RETRIES = 3;

  /**
   * 楽観ロック付きテキスト挿入（推奨）
   */
  async insertTextWithLock(
    documentId: string,
    text: string,
    position: number
  ): Promise<Result<void, ApiError | ConflictError>> {
    return await this.insertTextWithLockRetry(documentId, text, position, 0);
  }

  private async insertTextWithLockRetry(
    documentId: string,
    text: string,
    position: number,
    retryCount: number
  ): Promise<Result<void, ApiError | ConflictError>> {
    // 1. 現在のドキュメント状態とリビジョンIDを取得
    const docResult = await this.getDocument(documentId);
    if (docResult.isErr) return docResult;

    const doc = docResult.value;
    const revisionId = doc.revisionId;

    // 2. writeControlで楽観ロック
    const result = await this.batchUpdate(
      documentId,
      [{ insertText: { location: { index: position }, text } }],
      { requiredRevisionId: revisionId }
    );

    // 3. 競合検出: 400エラー + revision関連メッセージ
    if (result.isErr && result.error.status === 400) {
      if (result.error.message.includes('revision')) {
        // 最大リトライ回数チェック
        if (retryCount >= this.MAX_CONFLICT_RETRIES) {
          return Err({
            type: 'RevisionMismatch',
            status: 400,
            message: `Revision conflict after ${retryCount} retries`,
            retryCount
          });
        }

        // リトライ: ドキュメント再取得 → カーソル再計算 → 再挿入
        logger.warn('Revision conflict detected, retrying...', {
          documentId,
          retryCount: retryCount + 1
        });

        // カーソル位置を再計算（Named Rangeから取得）
        const newPosition = await this.recalculateCursorPosition(documentId);
        return await this.insertTextWithLockRetry(
          documentId,
          text,
          newPosition,
          retryCount + 1
        );
      }
    }

    return result;
  }

  private async recalculateCursorPosition(documentId: string): Promise<number> {
    const doc = await this.getDocument(documentId);
    const cursorRange = doc.value.namedRanges['transcript_cursor'];

    if (cursorRange) {
      return cursorRange.ranges[0].startIndex;
    }

    // Named Rangeが存在しない場合は復旧ロジック実行
    return await namedRangeManager.recoverNamedRange(documentId);
  }

  /**
   * テキスト挿入後のカーソル位置を厳密取得（推奨）
   *
   * 重要: 手計算（oldPosition + text.length）は不正確。
   *       APIレスポンスから厳密な位置を取得する。
   */
  private async getInsertedPosition(response: BatchUpdateResponse): Promise<number> {
    // batchUpdateレスポンスのreplies配列から挿入後のインデックスを取得
    const insertReply = response.replies[0]?.insertText;
    if (insertReply && insertReply.endIndex) {
      return insertReply.endIndex;
    }

    // フォールバック: レスポンスにendIndexが含まれない場合
    throw new Error('Failed to get inserted position from API response');
  }
}
```

**Error Handling Strategy**:
```typescript
async function handleApiError(error: ApiError): Promise<void> {
  switch (error.type) {
    case 'Unauthorized':
      // トークンリフレッシュを試行
      await authManager.refreshToken();
      // リトライ
      break;

    case 'RateLimitExceeded':
      // Exponential Backoffでリトライ
      await exponentialBackoff(() => retryRequest(), 5);
      break;

    case 'Forbidden':
      // ユーザーに権限エラーを通知
      showError('ドキュメントへのアクセス権限がありません');
      break;

    case 'NotFound':
      // リトライせず、キューから削除
      await queueManager.removeMessage(messageId);
      break;

    case 'ServerError':
      // Exponential Backoffでリトライ
      await exponentialBackoff(() => retryRequest(), 3);
      break;
  }
}
```

---

#### NamedRangeManager

**Responsibility & Boundaries**
- **Primary Responsibility**: Named Range (`transcript_cursor`) の作成、更新、位置取得、自動復旧
- **Domain Boundary**: API Domainのドキュメント構造管理層
- **Data Ownership**: Named Rangeの位置情報とメタデータ
- **Transaction Boundary**: Named Range操作の原子性（作成/更新/削除）

**Dependencies**
- **Inbound**: SyncManager、GoogleDocsClient
- **Outbound**: GoogleDocsClient（`getDocument`, `batchUpdate`）
- **External**: Google Docs API

**Contract Definition**:
```typescript
interface NamedRangeManager {
  /**
   * Named Rangeを作成する
   *
   * @preconditions ドキュメントにNamed Rangeが存在しない
   * @postconditions `transcript_cursor` Named Rangeが作成される
   * @throws CreateError 作成失敗
   */
  createNamedRange(documentId: string, position: number): Promise<Result<void, CreateError>>;

  /**
   * Named Rangeの現在位置を取得する
   *
   * @postconditions Named Rangeが存在しない場合は自動復旧ロジックを実行
   * @throws GetError 取得失敗
   */
  getCurrentPosition(documentId: string): Promise<Result<number, GetError>>;

  /**
   * Named Rangeの位置を更新する
   *
   * @preconditions Named Rangeが存在
   * @postconditions 新しい位置に更新される
   * @throws UpdateError 更新失敗
   */
  updatePosition(documentId: string, newPosition: number): Promise<Result<void, UpdateError>>;

  /**
   * Named Rangeが消失した場合の自動復旧ロジック
   *
   * @postconditions 段階的フォールバック戦略でNamed Rangeを再作成
   * @throws RecoveryError 復旧失敗
   */
  recoverNamedRange(documentId: string): Promise<Result<number, RecoveryError>>;
}

type CreateError = { type: 'AlreadyExists' | 'ApiError'; message: string };
type GetError = { type: 'NotFound' | 'ApiError'; message: string };
type UpdateError = { type: 'NotFound' | 'ApiError'; message: string };
type RecoveryError = { type: 'NoValidPosition'; message: string };
```

**Recovery Strategy** (DOCS-REQ-003.7-8):

**重要**: Markdown表現（"## 文字起こし"）ではなく、Google Docs APIの段落スタイル（`paragraphStyle.namedStyleType`）で見出しを検出する。

```typescript
async function recoverNamedRange(documentId: string): Promise<number> {
  // Priority 1: Search for "文字起こし" heading by paragraph style (HEADING_2)
  const doc = await googleDocsClient.getDocument(documentId);
  const headingIndex = findHeadingByStyle(doc, 'HEADING_2', '文字起こし');

  if (headingIndex !== null) {
    logger.error('Named Range消失 - 見出し後に再作成', { documentId, position: headingIndex });
    await createNamedRange(documentId, headingIndex);
    showNotification('挿入位置が再設定されました');
    return headingIndex;
  }

  // Priority 2: Document end
  const endIndex = doc.body.content[doc.body.content.length - 1].endIndex - 1;
  if (endIndex > 1) {
    logger.error('Named Range消失 - 末尾に再作成', { documentId, position: endIndex });
    await createNamedRange(documentId, endIndex);
    showNotification('挿入位置が再設定されました');
    return endIndex;
  }

  // Priority 3: Document start
  logger.error('Named Range消失 - 先頭に再作成', { documentId, position: 1 });
  await createNamedRange(documentId, 1);
  showNotification('挿入位置が再設定されました');
  return 1;
}

/**
 * 段落スタイルで見出しを検索（堅牢な実装）
 *
 * @param doc Google Docsドキュメント
 * @param style 見出しスタイル（例: 'HEADING_2'）
 * @param text 検索するテキスト
 * @returns 見出しの直後のインデックス、見つからない場合はnull
 */
function findHeadingByStyle(doc: Document, style: string, text: string): number | null {
  for (const element of doc.body.content) {
    // 段落スタイルをチェック
    if (element.paragraph?.paragraphStyle?.namedStyleType === style) {
      // 段落内のテキストをチェック
      for (const textElement of element.paragraph.elements) {
        if (textElement.textRun?.content?.includes(text)) {
          // 見出しの直後のインデックスを返す
          return element.endIndex;
        }
      }
    }
  }
  return null;
}
```

---

