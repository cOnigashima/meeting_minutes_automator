# Technical Design - meeting-minutes-docs-sync: Testing & Security

> **プロジェクト**: OAuth 2.0 + Google Docs API統合でGoogle Meetから議事録を自動同期（MVP2）
> **親ドキュメント**: [design.md](../design.md)
> **関連**: [Requirements](../requirements.md) | [Tasks](../tasks.md) | [他のモジュール](README.md)

## Testing Strategy

### Unit Tests

1. **AuthManager.initiateAuth()**: OAuth 2.0認証フローの正常系とエラー系
   - 正常系: トークン取得成功 → TokenStoreに保存
   - エラー系: ユーザーキャンセル、ネットワークエラー、Invalid Grant

2. **AuthManager.refreshToken()**: トークンリフレッシュの正常系とエラー系
   - 正常系: リフレッシュトークン有効 → 新しいアクセストークン取得
   - エラー系: リフレッシュトークン無効 → 再認証プロンプト

3. **QueueManager.enqueue()**: オフラインキューへのメッセージ追加
   - 正常系: ストレージ容量内 → メッセージ保存
   - エラー系: ストレージ上限到達 → QueueFullError

4. **GoogleDocsClient.batchUpdate()**: Google Docs APIリクエストの正常系とエラー系
   - 正常系: 200 OK → 成功
   - エラー系: 401, 403, 429, 500-504 → 適切なエラーハンドリング

5. **NamedRangeManager.recoverNamedRange()**: Named Range自動復旧ロジック
   - Priority 1: 見出し検索成功 → 見出し直後に再作成
   - Priority 2: ドキュメント末尾に再作成
   - Priority 3: 先頭に再作成

### Integration Tests

1. **OAuth 2.0認証フロー + トークンリフレッシュ**:
   - シナリオ: 認証 → トークン保存 → 期限切れ → 自動リフレッシュ → API呼び出し成功
   - 検証: `chrome.storage.local`にトークンが保存され、APIリクエストに正しいトークンが使用される

2. **オンライン同期フロー**:
   - シナリオ: 文字起こしメッセージ受信 → バッファリング → Google Docs挿入 → 成功通知
   - 検証: Google Docsにテキストが挿入され、`docs_sync_success`イベントがTauriアプリへ送信される

3. **オフライン → オンライン復帰フロー**:
   - シナリオ: ネットワーク切断 → メッセージキュー保存 → ネットワーク復帰 → 自動再同期
   - 検証: キュー内のメッセージが時系列順に再送信され、Google Docsに反映される

4. **Named Range消失 → 自動復旧**:
   - シナリオ: Named Range削除 → 挿入試行 → 自動復旧ロジック実行 → 正常挿入
   - 検証: ERRORログ記録 + UI通知 + 正しい位置にNamed Range再作成

5. **レート制限エラー → Exponential Backoff**:
   - シナリオ: 短時間に大量メッセージ送信 → 429エラー → Exponential Backoffでリトライ → 成功
   - 検証: リトライ間隔が指数関数的に増加し、最終的に成功する

---

## Security Considerations

### 認証とトークン管理

**OAuth 2.0スコープ選択**:
- **使用スコープ**: `drive.file` (Non-sensitive Scope)
- **理由**: アプリで作成/明示的に共有されたファイルのみにアクセス。Google OAuth審査不要。
- **代替案**: `drive`（Sensitive Scope）は全ファイルへのアクセス権を要求し、ユーザーの信頼を損なうリスクあり

**トークン保存の脆弱性** (DOCS-NFR-003.1):
- **MVP2での制約**: `chrome.storage.local`は暗号化されていない。トークンが平文で保存される
- **リスク**: マルウェアによるストレージアクセスでトークンが漏洩する可能性
- **MVP2での緩和策**:
  ```typescript
  // 1. アクセストークンの有効期限を短縮（1時間 → 30分）
  const expiresAt = Math.floor(Date.now() / 1000) + 1800; // 30分（1800秒）

  // 2. Service Workerサスペンド時の自動ログアウト
  chrome.runtime.onSuspend.addListener(async () => {
    await chrome.storage.local.remove(['accessToken', 'refreshToken']);
    logger.info('Tokens cleared on Service Worker suspend');
  });

  // 3. セキュリティ警告の表示
  function showSecurityWarning(): void {
    showNotification({
      type: 'warning',
      title: 'セキュリティに関する注意',
      message: 'トークンはローカルストレージに保存されます。共有PCでの使用は避けてください。'
    });
  }
  ```
- **MVP3での改善**: Tauri側（OSキーチェーン）でトークン管理、またはWeb Crypto API（`crypto.subtle`）で暗号化

**トークン無効化** (DOCS-NFR-003.4):
- **実装**: ユーザーが「Google連携解除」を実行時、Googleへトークン無効化リクエストを送信
  ```typescript
  async function revokeToken(accessToken: string): Promise<void> {
    await fetch(`https://oauth2.googleapis.com/revoke?token=${accessToken}`, {
      method: 'POST',
    });
    await chrome.storage.local.remove('auth_tokens');
  }
  ```

---

### API通信のセキュリティ

**HTTPS強制** (DOCS-NFR-003.2):
- **実装**: Google Docs API通信は全てHTTPSプロトコルを使用
- **検証**: `https://docs.googleapis.com`への通信のみ許可

**Authorization Headerの保護** (DOCS-NFR-003.3):
- **実装**: アクセストークンを`Authorization: Bearer [token]`ヘッダーに設定
- **検証**: トークンをURLパラメータに含めない（ログ漏洩リスク回避）

**CSP (Content Security Policy)** (manifest.json):
```json
{
  "content_security_policy": {
    "extension_pages": "script-src 'self'; object-src 'self'; connect-src https://docs.googleapis.com https://oauth2.googleapis.com"
  }
}
```
- **Purpose**: 外部スクリプトの実行を禁止し、XSS攻撃を防止

---

### 入力バリデーション

**ドキュメントID検証**:

**重要**: 44文字固定の正規表現は誤り。Google Docs IDは可変長（通常25文字以上）。

```typescript
function isValidDocumentId(documentId: string): boolean {
  // Google Docs document IDは可変長（25文字以上）の英数字とハイフン、アンダースコア
  const regex = /^[a-zA-Z0-9_-]{25,}$/;

  // 基本的な形式チェック
  if (!regex.test(documentId)) {
    return false;
  }

  // 実際のリクエストで最終検証
  // （形式が正しくてもドキュメントが存在しない可能性があるため）
  return true;
}
```

**テキスト挿入のサニタイゼーション**:
```typescript
function sanitizeText(text: string): string {
  // 制御文字を削除（改行、タブは許可）
  return text.replace(/[\x00-\x08\x0B-\x0C\x0E-\x1F\x7F]/g, '');
}
```

---

## Performance & Scalability

### Target Metrics

- **文字起こしメッセージ受信 → Google Docs挿入完了**: 2秒以内（DOCS-NFR-001.1）
- **Google Docs API応答時間**: 95パーセンタイルで3秒以内（DOCS-NFR-001.2）
- **オフラインキュー再送信**: 100メッセージあたり最大120秒（DOCS-NFR-001.3）
- **ローカルストレージ書き込み**: 10ms以内（DOCS-NFR-001.4）

---

### Scaling Approaches

#### レート制限管理（Token Bucket RateLimiter）

**目的**: Google Docs APIレート制限（60リクエスト/分/ユーザー）を遵守し、NFR「2秒以内」を維持しつつAPI呼び出しを吸収する。

**実装**: Token Bucket アルゴリズム

```typescript
class TokenBucketRateLimiter {
  private tokens: number;
  private readonly capacity: number = 60; // 60 tokens/min
  private readonly refillRate: number = 1; // 1 token/sec
  private lastRefillTime: number = Date.now();

  constructor() {
    this.tokens = this.capacity;
  }

  /**
   * トークンを取得（API呼び出し前に実行）
   */
  async acquire(): Promise<void> {
    await this.refill();

    if (this.tokens >= 1) {
      this.tokens -= 1;
      return;
    }

    // トークン不足: 次のリフィルまで待機
    const waitTime = 1000; // 1秒待機
    logger.warn('Rate limit: waiting for token', { waitTime, remainingTokens: this.tokens });
    await sleep(waitTime);
    return await this.acquire(); // 再試行
  }

  /**
   * トークンをリフィル（時間経過に応じて補充）
   */
  private async refill(): Promise<void> {
    const now = Date.now();
    const elapsed = now - this.lastRefillTime;
    const tokensToAdd = Math.floor(elapsed / 1000) * this.refillRate;

    this.tokens = Math.min(this.tokens + tokensToAdd, this.capacity);
    this.lastRefillTime = now;
  }

  /**
   * 現在のトークン数を取得（デバッグ用）
   */
  getAvailableTokens(): number {
    return Math.floor(this.tokens);
  }
}

// 使用例
const rateLimiter = new TokenBucketRateLimiter();

async function callGoogleDocsAPI(documentId: string, requests: Request[]): Promise<void> {
  // API呼び出し前にトークンを取得
  await rateLimiter.acquire();

  // Google Docs API呼び出し
  await googleDocsClient.batchUpdate(documentId, requests);
}
```

**効果**:
- レート制限（60リクエスト/分）を確実に遵守
- NFR「2秒以内」を維持（バッファリング時間は別プロセス）
- 429エラー（Rate Limit Exceeded）の発生を防止

---

#### バッファリング戦略 (DOCS-REQ-004.6-7)

**目的**: Google Docs APIレート制限（60リクエスト/分/ユーザー）を遵守し、API呼び出し回数を削減

**重要**: MV3 Service Workerは5分でタイムアウトし、`setTimeout`/`setInterval`は不安定なため、`chrome.alarms` APIを使用する。

**実装**:
```typescript
class BufferingManager {
  private buffer: TranscriptionMessage[] = [];
  private readonly MAX_BUFFER_TIME_MS = 3000; // 3秒
  private readonly MAX_BUFFER_SIZE = 500;   // 500文字
  private readonly ALARM_NAME = 'flush-buffer';

  async addToBuffer(message: TranscriptionMessage): Promise<void> {
    this.buffer.push(message);

    // 文字数チェック
    const totalChars = this.buffer.reduce((sum, m) => sum + m.text.length, 0);
    if (totalChars >= this.MAX_BUFFER_SIZE) {
      await this.flush();
      return;
    }

    // chrome.alarms でタイマー設定（初回メッセージ時のみ）
    const existingAlarm = await chrome.alarms.get(this.ALARM_NAME);
    if (!existingAlarm) {
      // 3秒後にフラッシュ（0.05分 = 3秒）
      await chrome.alarms.create(this.ALARM_NAME, { delayInMinutes: 0.05 });
    }
  }

  async flush(): Promise<void> {
    // アラームをクリア
    await chrome.alarms.clear(this.ALARM_NAME);

    if (this.buffer.length === 0) return;

    const messages = [...this.buffer];
    this.buffer = [];

    // 複数メッセージを1つの batchUpdate リクエストにまとめる
    await googleDocsClient.insertBatchMessages(messages);
  }
}

// Service Worker (background.js) でアラームリスナーを設定
chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === 'flush-buffer') {
    bufferingManager.flush();
  }
});
```

**効果**:
- API呼び出し回数を最大60%削減（1メッセージ/1リクエスト → 複数メッセージ/1リクエスト）
- レート制限エラー（429）の発生頻度を低減
- MV3 Service Workerのタイムアウト問題を回避

---

#### オフラインキュー最適化

**ストレージ使用量の監視** (DOCS-REQ-005.11-12):

**重要**: MV3 Service Workerの`setInterval`は不安定なため、`chrome.alarms`で定期実行する。

```typescript
async function monitorStorageUsage(): Promise<void> {
  const STORAGE_LIMIT = 10 * 1024 * 1024; // 10 MB
  const usage = await chrome.storage.local.getBytesInUse();
  const usageRatio = usage / STORAGE_LIMIT;

  if (usageRatio >= 0.8) {
    showPopupWarning(`オフラインキューが残り${Math.floor((1 - usageRatio) * 100)}%です`);
  }

  if (usageRatio >= 1.0) {
    chrome.notifications.create({
      type: 'basic',
      iconUrl: 'icon.png',
      title: 'オフラインキュー上限到達',
      message: 'これ以上の文字起こしは保存されません',
      priority: 2,
    });
    stopReceivingMessages();
  }
}

// chrome.alarms で定期監視（6秒間隔 = 0.1分）
chrome.alarms.create('monitor-storage', { periodInMinutes: 0.1 });

chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === 'monitor-storage') {
    monitorStorageUsage();
  }
});
```

**キュー圧縮** (将来拡張):
```typescript
// 連続した短いメッセージを1つにマージ
function compressQueue(queue: QueueItem[]): QueueItem[] {
  const compressed: QueueItem[] = [];
  let current: QueueItem | null = null;

  for (const item of queue) {
    if (current && item.message.text.length < 50) {
      // 短いメッセージをマージ
      current.message.text += '\n' + item.message.text;
    } else {
      if (current) compressed.push(current);
      current = item;
    }
  }

  if (current) compressed.push(current);
  return compressed;
}
```

---

### Caching Strategies

**トークンキャッシュ**:
- アクセストークンをメモリにキャッシュし、`chrome.storage.local`への頻繁なアクセスを回避
- 有効期限60秒前に自動リフレッシュ

**Named Range位置キャッシュ**:
- Named Rangeの位置をメモリにキャッシュし、`documents.get`リクエスト回数を削減
- テキスト挿入ごとにキャッシュを更新

---

