# Interface Contracts - All Domains

> **親ドキュメント**: [phase-0-design-validation.md](../task-details/phase-0-design-validation.md)
> **関連**: [責務マトリクス](responsibility-matrix.md)

## Overview

全19クラスのTypeScriptインターフェース定義。各メソッドに事前条件/事後条件/エラー型を記載。

**注**: 本ドキュメントはスケルトン版です。Phase 0, Task 0.3.1で各インターフェースの完全な契約定義を追加します。

---

## Auth Domain Interfaces

### IChromeIdentityClient

```typescript
/**
 * Chrome Identity APIの抽象化インターフェース
 *
 * 責務: Chrome Identity APIの低レベル呼び出しをカプセル化
 *
 * テスト戦略: モックオブジェクトで完全にスタブ可能
 */
export interface IChromeIdentityClient {
  /**
   * OAuth 2.0認証フローを開始する
   *
   * @preconditions なし
   * @postconditions 認証コードが返される
   * @throws UserCancelledError ユーザーがキャンセル
   * @throws NetworkError ネットワークエラー
   * @returns Result<認証コード, AuthFlowError>
   */
  launchAuthFlow(): Promise<Result<string, AuthFlowError>>;

  /**
   * 認証コードをアクセストークンに交換する
   *
   * @preconditions code が有効な認証コード
   * @postconditions TokenResponse が返される
   * @throws InvalidGrantError 認証コードが無効
   * @throws NetworkError ネットワークエラー
   * @returns Result<TokenResponse, TokenExchangeError>
   */
  exchangeCodeForToken(code: string): Promise<Result<TokenResponse, TokenExchangeError>>;
}
```

### ITokenStore

```typescript
/**
 * トークンストレージの抽象化インターフェース
 *
 * 責務: トークンの永続化のみ（検証ロジックは含まない）
 *
 * テスト戦略: インメモリ実装で完全にモック可能
 */
export interface ITokenStore {
  /**
   * トークンを保存する
   *
   * @preconditions token が有効な AuthToken
   * @postconditions chrome.storage.local に保存される
   * @throws StorageFullError ストレージ上限到達
   */
  save(token: AuthToken): Promise<Result<void, StorageError>>;

  /**
   * トークンを読み込む
   *
   * @preconditions なし
   * @postconditions トークンが存在すれば返される
   */
  load(): Promise<AuthToken | null>;

  /**
   * トークンを削除する
   *
   * @preconditions なし
   * @postconditions chrome.storage.local からトークンが削除される
   */
  remove(): Promise<void>;
}
```

### IAuthManager

```typescript
/**
 * OAuth 2.0認証フロー統合インターフェース
 *
 * 責務: 認証フロー実行とトークンライフサイクル管理
 *
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */
export interface IAuthManager {
  /**
   * OAuth 2.0認証フローを開始し、トークンを取得
   *
   * @preconditions chrome.identity APIが利用可能
   * @postconditions トークンがTokenStoreに保存される
   * @throws AuthError トークン取得失敗、ユーザーキャンセル
   */
  initiateAuth(): Promise<Result<AuthTokens, AuthError>>;

  /**
   * アクセストークンを取得（期限切れ時は自動リフレッシュ）
   *
   * @preconditions TokenStoreにリフレッシュトークンが保存されている
   * @postconditions 有効なアクセストークンが返される
   * @throws TokenExpiredError リフレッシュトークンも無効
   */
  getAccessToken(): Promise<Result<string, TokenExpiredError>>;

  /**
   * トークンを無効化し、TokenStoreから削除
   *
   * @postconditions TokenStoreが空になる
   * @throws RevokeError トークン無効化失敗（ベストエフォート）
   */
  revokeToken(): Promise<Result<void, RevokeError>>;
}
```

### ITokenRefresher

```typescript
/**
 * トークンリフレッシュロジックインターフェース
 *
 * 責務: リフレッシュトークンを使用した新しいアクセストークン取得
 *
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */
export interface ITokenRefresher {
  /**
   * リフレッシュトークンを使用してアクセストークンを更新
   *
   * @preconditions refreshToken が有効
   * @postconditions 新しいアクセストークンが返される
   * @throws RefreshError リフレッシュトークンが無効
   */
  refreshAccessToken(refreshToken: string): Promise<Result<string, RefreshError>>;

  /**
   * トークン有効期限の監視を開始（chrome.alarms使用）
   *
   * @preconditions expiresAt が未来の日時
   * @postconditions 有効期限60秒前にアラームが設定される
   */
  startExpiryMonitor(expiresAt: number): Promise<void>;
}
```

### ITokenExpiryMonitor

```typescript
/**
 * chrome.alarms管理インターフェース
 *
 * 責務: トークン有効期限監視とアラーム管理
 *
 * テスト戦略: Chrome API モック化必要（⭐⭐⭐）
 */
export interface ITokenExpiryMonitor {
  /**
   * トークン有効期限監視アラームを作成
   *
   * @preconditions expiresAt が未来の日時
   * @postconditions chrome.alarmsにアラームが登録される
   */
  createAlarm(expiresAt: number): Promise<void>;

  /**
   * アラームを削除
   *
   * @postconditions chrome.alarmsからアラームが削除される
   */
  clearAlarm(): Promise<void>;
}
```

---

## Sync Domain Interfaces

### ISyncManager

```typescript
/**
 * 同期フロー統合インターフェース
 *
 * 責務: 文字起こしメッセージ受信、オンライン/オフライン状態管理、自動同期制御
 *
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */
export interface ISyncManager {
  /**
   * Google Docs同期を開始
   *
   * @preconditions AuthManagerで認証済み、Google Docsタブがアクティブ
   * @postconditions ドキュメントIDが取得され、Named Rangeが作成される
   * @throws SyncStartError ドキュメントID取得失敗、Named Range作成失敗
   */
  startSync(documentId: string): Promise<Result<void, SyncStartError>>;

  /**
   * 文字起こしメッセージを処理
   *
   * @preconditions startSync()実行済み
   * @postconditions オンライン時はGoogleDocsClientへ送信、オフライン時はQueueManagerへ保存
   */
  processTranscription(message: TranscriptionMessage): Promise<Result<void, ProcessError>>;

  /**
   * ネットワーク復帰時にオフラインキューを再同期
   *
   * @preconditions オフラインキューにメッセージが存在
   * @postconditions 全メッセージの送信完了後にキューがクリアされる
   * @throws ResyncError 再送信中のエラー
   */
  resyncOfflineQueue(): Promise<Result<void, ResyncError>>;

  /**
   * Google Docs同期を停止
   *
   * @postconditions 同期ステータスがリセットされる
   */
  stopSync(): Promise<void>;
}
```

### ISyncStateMachine

```typescript
/**
 * 同期状態遷移管理インターフェース
 *
 * 責務: 同期状態（NotStarted/Syncing/Paused/Error）の遷移ロジック
 *
 * テスト戦略: 完全にモック可能（⭐⭐⭐⭐⭐）
 */
export interface ISyncStateMachine {
  /**
   * 現在の同期状態を取得
   *
   * @postconditions 現在の状態を返す
   */
  getCurrentState(): SyncState;

  /**
   * 状態を遷移
   *
   * @preconditions 遷移が有効（例: NotStarted → Syncing）
   * @postconditions 状態が更新される
   * @throws InvalidTransitionError 無効な遷移
   */
  transition(toState: SyncState): Result<void, InvalidTransitionError>;

  /**
   * 状態をリセット（NotStartedに戻す）
   *
   * @postconditions 状態がNotStartedになる
   */
  reset(): void;
}
```

### IQueueManager

```typescript
/**
 * オフラインキュー操作インターフェース
 *
 * 責務: オフライン時のメッセージキューイング、FIFO順で送信
 *
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */
export interface IQueueManager {
  /**
   * メッセージをキューに追加
   *
   * @preconditions chrome.storage.localが利用可能
   * @postconditions メッセージがストレージに保存される
   * @throws StorageFullError ストレージ上限到達
   */
  enqueue(message: TranscriptionMessage): Promise<Result<void, StorageFullError>>;

  /**
   * キューから全メッセージを取得
   *
   * @postconditions FIFO順でメッセージリストを返す
   */
  dequeueAll(): Promise<TranscriptionMessage[]>;

  /**
   * キューをクリア
   *
   * @postconditions ストレージからキューが削除される
   */
  clear(): Promise<void>;

  /**
   * キューサイズを取得
   *
   * @postconditions 現在のキューサイズを返す
   */
  size(): Promise<number>;
}
```

### IStorageMonitor

```typescript
/**
 * ストレージ監視インターフェース
 *
 * 責務: chrome.storage.localの使用状況監視、警告通知
 *
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */
export interface IStorageMonitor {
  /**
   * ストレージ使用率を取得
   *
   * @postconditions 0-100の使用率パーセンテージを返す
   */
  getUsagePercentage(): Promise<number>;

  /**
   * ストレージ監視を開始（chrome.alarms使用、15分間隔）
   *
   * @postconditions chrome.alarmsにアラームが登録される
   */
  startMonitoring(): Promise<void>;

  /**
   * ストレージ監視を停止
   *
   * @postconditions chrome.alarmsからアラームが削除される
   */
  stopMonitoring(): Promise<void>;
}
```

### IBufferingManager

```typescript
/**
 * バッファリング管理インターフェース
 *
 * 責務: 短時間のメッセージをバッファリング、一括送信
 *
 * テスト戦略: 完全にモック可能（⭐⭐⭐⭐⭐）
 */
export interface IBufferingManager {
  /**
   * メッセージをバッファに追加
   *
   * @postconditions メッセージがバッファに追加され、タイマーが開始される
   */
  buffer(message: TranscriptionMessage): void;

  /**
   * バッファをフラッシュ（即座送信）
   *
   * @postconditions 全メッセージがGoogleDocsClientへ送信される
   */
  flush(): Promise<void>;

  /**
   * バッファをクリア
   *
   * @postconditions バッファが空になる
   */
  clear(): void;
}
```

### ITokenBucketRateLimiter

```typescript
/**
 * レート制限制御インターフェース
 *
 * 責務: Token Bucketアルゴリズムでレート制限（60リクエスト/分）
 *
 * テスト戦略: 完全にモック可能（⭐⭐⭐⭐⭐）
 */
export interface ITokenBucketRateLimiter {
  /**
   * リクエスト許可を取得
   *
   * @postconditions トークンが利用可能な場合は即座返却、不可の場合は待機
   */
  acquire(): Promise<void>;

  /**
   * 現在の利用可能トークン数を取得
   *
   * @postconditions 0-60のトークン数を返す
   */
  getAvailableTokens(): number;
}
```

### INetworkMonitor

```typescript
/**
 * ネットワーク監視インターフェース
 *
 * 責務: オンライン/オフライン検知、状態変更通知
 *
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */
export interface INetworkMonitor {
  /**
   * 現在のネットワーク状態を取得
   *
   * @postconditions true（オンライン）またはfalse（オフライン）を返す
   */
  isOnline(): boolean;

  /**
   * ネットワーク状態変更時のコールバックを登録
   *
   * @postconditions オンライン/オフライン変更時にコールバックが実行される
   */
  onStateChange(callback: (isOnline: boolean) => void): void;

  /**
   * コールバックを解除
   *
   * @postconditions 登録されたコールバックが削除される
   */
  removeStateChangeListener(): void;
}
```

### IResyncOrchestrator

```typescript
/**
 * 再同期制御インターフェース
 *
 * 責務: オフラインキュー再同期の制御、レート制限遵守
 *
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */
export interface IResyncOrchestrator {
  /**
   * オフラインキューを再同期
   *
   * @preconditions ネットワークがオンライン
   * @postconditions 全メッセージの送信完了後にQueueManagerをクリア
   * @throws ResyncError 再送信中のエラー
   */
  resync(): Promise<Result<void, ResyncError>>;
}
```

---

## API Domain Interfaces

### IGoogleDocsClient

```typescript
/**
 * Google Docs API呼び出し統合インターフェース
 *
 * 責務: documents.batchUpdate呼び出し、リトライ、楽観ロック
 *
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */
export interface IGoogleDocsClient {
  /**
   * テキストを挿入
   *
   * @preconditions documentId が有効、accessToken が有効
   * @postconditions Google Docsにテキストが挿入される
   * @throws ApiError API呼び出し失敗（401/403/429/500）
   */
  insertText(documentId: string, text: string, index: number): Promise<Result<void, ApiError>>;

  /**
   * Named Rangeを作成
   *
   * @preconditions documentId が有効、accessToken が有効
   * @postconditions Named Rangeが作成される
   * @throws ApiError API呼び出し失敗
   */
  createNamedRange(documentId: string, name: string, startIndex: number, endIndex: number): Promise<Result<void, ApiError>>;

  /**
   * Named Rangeの位置を取得
   *
   * @preconditions Named Rangeが存在
   * @postconditions Named Rangeの位置情報を返す
   * @throws NotFoundError Named Rangeが存在しない
   */
  getNamedRangePosition(documentId: string, name: string): Promise<Result<{ startIndex: number; endIndex: number }, NotFoundError>>;
}
```

### IExponentialBackoffHandler

```typescript
/**
 * Exponential Backoffリトライ戦略インターフェース
 *
 * 責務: 429 Too Many Requests時のリトライロジック
 *
 * テスト戦略: 完全にモック可能（⭐⭐⭐⭐⭐）
 */
export interface IExponentialBackoffHandler {
  /**
   * リトライ付きでAPI呼び出しを実行
   *
   * @preconditions fn が非同期関数
   * @postconditions 成功するまでリトライ（1秒、2秒、4秒、最大3回）
   * @throws MaxRetriesExceededError 最大リトライ回数超過
   */
  executeWithBackoff<T>(fn: () => Promise<T>): Promise<Result<T, MaxRetriesExceededError>>;
}
```

### IOptimisticLockHandler

```typescript
/**
 * 楽観ロック制御インターフェース
 *
 * 責務: writeControl.requiredRevisionIdによる楽観ロック
 *
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */
export interface IOptimisticLockHandler {
  /**
   * 楽観ロック付きでbatchUpdateを実行
   *
   * @preconditions documentId が有効、revisionId が最新
   * @postconditions 成功時は新しいrevisionIdを返す
   * @throws ConflictError 楽観ロック失敗（revisionId不一致）
   */
  batchUpdateWithLock(documentId: string, requests: any[], revisionId: string): Promise<Result<string, ConflictError>>;
}
```

### INamedRangeManager

```typescript
/**
 * Named Range統合インターフェース
 *
 * 責務: Named Range作成、更新、自動復旧
 *
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */
export interface INamedRangeManager {
  /**
   * Named Rangeを作成（同期カーソル）
   *
   * @preconditions documentId が有効
   * @postconditions Named Rangeが作成される
   * @throws ApiError API呼び出し失敗
   */
  initializeCursor(documentId: string): Promise<Result<void, ApiError>>;

  /**
   * Named Rangeを更新（挿入位置移動）
   *
   * @preconditions Named Rangeが存在
   * @postconditions Named Rangeの位置が更新される
   * @throws ApiError API呼び出し失敗
   */
  updateCursorPosition(documentId: string, newIndex: number): Promise<Result<void, ApiError>>;

  /**
   * Named Rangeを自動復旧（削除時）
   *
   * @preconditions Named Rangeが削除されている
   * @postconditions Named Rangeが再作成される（ドキュメント末尾）
   * @throws ApiError API呼び出し失敗
   */
  recoverCursor(documentId: string): Promise<Result<void, ApiError>>;
}
```

### INamedRangeRecoveryStrategy

```typescript
/**
 * Named Range自動復旧戦略インターフェース
 *
 * 責務: Named Range削除検知、自動復旧ロジック
 *
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */
export interface INamedRangeRecoveryStrategy {
  /**
   * Named Rangeの復旧を実行
   *
   * @preconditions Named Rangeが削除されている
   * @postconditions ドキュメント末尾にNamed Rangeを再作成
   * @throws ApiError API呼び出し失敗
   */
  recover(documentId: string): Promise<Result<void, ApiError>>;
}
```

### IParagraphStyleFormatter

```typescript
/**
 * 段落スタイル設定インターフェース
 *
 * 責務: 見出し、タイムスタンプ、話者名のスタイル設定
 *
 * テスト戦略: 完全にモック可能（⭐⭐⭐⭐⭐）
 */
export interface IParagraphStyleFormatter {
  /**
   * 見出しスタイルを生成
   *
   * @postconditions HEADING_1スタイルのbatchUpdateリクエストを返す
   */
  formatHeading(text: string): any;

  /**
   * タイムスタンプスタイルを生成
   *
   * @postconditions NORMAL_TEXT + 太字スタイルのbatchUpdateリクエストを返す
   */
  formatTimestamp(text: string): any;

  /**
   * 話者名スタイルを生成
   *
   * @postconditions NORMAL_TEXT + 太字 + 下線スタイルのbatchUpdateリクエストを返す
   */
  formatSpeakerName(text: string): any;
}
```

---

## Result Type Definition

```typescript
/**
 * Result型: 成功/失敗を表現する型
 */
export type Result<T, E> = Ok<T> | Err<E>;

export type Ok<T> = {
  isOk: true;
  isErr: false;
  value: T;
};

export type Err<E> = {
  isOk: false;
  isErr: true;
  error: E;
};
```

---

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-10-24 | 0.1 | Claude Code | スケルトン版作成（Auth Domain 2インターフェース） |
| 2025-10-30 | 1.0 | Claude Code | 完全版（全19インターフェース契約定義完成） |
