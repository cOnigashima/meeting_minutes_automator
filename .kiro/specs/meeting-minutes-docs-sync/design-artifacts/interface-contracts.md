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

**Note**: 残り17インターフェースの詳細契約定義はPhase 0, Task 0.3.1で追加予定。

---

## Sync Domain Interfaces

### ISyncManager
**TODO**: Task 0.3.1で完全な契約定義を追加

### ISyncStateMachine
**TODO**: Task 0.3.1で完全な契約定義を追加

### IQueueManager
**TODO**: Task 0.3.1で完全な契約定義を追加

### IStorageMonitor
**TODO**: Task 0.3.1で完全な契約定義を追加

### IBufferingManager
**TODO**: Task 0.3.1で完全な契約定義を追加

### ITokenBucketRateLimiter
**TODO**: Task 0.3.1で完全な契約定義を追加

### INetworkMonitor
**TODO**: Task 0.3.1で完全な契約定義を追加

### IResyncOrchestrator
**TODO**: Task 0.3.1で完全な契約定義を追加

---

## API Domain Interfaces

### IGoogleDocsClient
**TODO**: Task 0.3.1で完全な契約定義を追加

### IExponentialBackoffHandler
**TODO**: Task 0.3.1で完全な契約定義を追加

### IOptimisticLockHandler
**TODO**: Task 0.3.1で完全な契約定義を追加

### INamedRangeManager
**TODO**: Task 0.3.1で完全な契約定義を追加

### INamedRangeRecoveryStrategy
**TODO**: Task 0.3.1で完全な契約定義を追加

### IParagraphStyleFormatter
**TODO**: Task 0.3.1で完全な契約定義を追加

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
