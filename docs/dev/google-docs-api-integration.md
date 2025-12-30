# Google Docs API Integration - Developer Guide

Meeting Minutes AutomatorのGoogle Docs同期機能の開発者向けドキュメント。

## 目次

1. [アーキテクチャ概要](#アーキテクチャ概要)
2. [セットアップ手順](#セットアップ手順)
3. [API仕様](#api仕様)
4. [テスト](#テスト)
5. [トラブルシューティング](#トラブルシューティング)

---

## アーキテクチャ概要

### システム構成

```
┌─────────────────────────────────────────────────────────────────┐
│                     Chrome Extension (MV3)                       │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐   ┌──────────────┐   ┌─────────────────────┐  │
│  │   Popup     │   │  Background  │   │  Offscreen Document │  │
│  │  (popup.ts) │   │ (background) │   │   (WebSocket持続)   │  │
│  └──────┬──────┘   └──────┬───────┘   └──────────┬──────────┘  │
│         │                 │                      │              │
│         └────────────────►├◄─────────────────────┘              │
│                           │                                     │
│  ┌────────────────────────┴────────────────────────────────┐   │
│  │                    Core Modules                          │   │
│  │  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐  │   │
│  │  │ AuthManager │  │ SyncManager  │  │ GoogleDocsClient│  │   │
│  │  └──────┬──────┘  └──────┬───────┘  └───────┬────────┘  │   │
│  │         │                │                   │           │   │
│  │  ┌──────┴──────┐  ┌──────┴───────┐  ┌───────┴────────┐  │   │
│  │  │ChromeIdentity│ │ QueueManager │  │NamedRangeManager│  │   │
│  │  │   Client    │  │ RateLimiter  │  │ RecoveryStrategy│  │   │
│  │  └─────────────┘  │ NetworkMonitor│  └────────────────┘  │   │
│  │                   │ StateMachine │                       │   │
│  │                   └──────────────┘                       │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
              │                              │
              │ WebSocket                    │ HTTPS
              ▼                              ▼
┌─────────────────────┐          ┌─────────────────────┐
│    Tauri App        │          │   Google APIs       │
│ (STT, Audio処理)    │          │ - OAuth 2.0         │
│ Port: 9001-9100     │          │ - Docs API v1       │
└─────────────────────┘          └─────────────────────┘
```

### データフロー

1. **文字起こし受信**: Tauri → WebSocket → Offscreen → Background
2. **同期処理**: Background → SyncManager → GoogleDocsClient → Google Docs API
3. **エラー時**: QueueManager → chrome.storage.local → 再送信

---

## セットアップ手順

### 1. Google Cloud Project作成

1. [Google Cloud Console](https://console.cloud.google.com/) にアクセス
2. 「新しいプロジェクト」を作成
3. プロジェクト名: `meeting-minutes-automator` (任意)

### 2. Google Docs API 有効化

1. 「APIとサービス」→「ライブラリ」
2. 「Google Docs API」を検索して有効化
3. 「Google Drive API」も有効化（ドキュメントアクセス用）

### 3. OAuth 2.0 認証情報作成

1. 「APIとサービス」→「認証情報」
2. 「認証情報を作成」→「OAuth クライアント ID」
3. アプリケーションの種類: 「Chrome 拡張機能」
4. 拡張機能ID: 開発用は `chrome://extensions` で確認

### 4. manifest.json 設定

```json
{
  "oauth2": {
    "client_id": "YOUR_CLIENT_ID.apps.googleusercontent.com",
    "scopes": [
      "https://www.googleapis.com/auth/documents",
      "https://www.googleapis.com/auth/drive.file"
    ]
  },
  "permissions": [
    "identity",
    "storage",
    "alarms",
    "offscreen",
    "notifications"
  ]
}
```

### 5. 開発環境起動

```bash
# Chrome拡張ビルド
cd chrome-extension
npm install
npm run build

# Chromeで読み込み
# chrome://extensions → デベロッパーモード → 「パッケージ化されていない拡張機能を読み込む」
# → chrome-extension/dist を選択
```

---

## API仕様

### AuthManager

認証状態管理とトークン取得を担当。

```typescript
interface IAuthManager {
  // アクセストークン取得（自動リフレッシュ）
  getAccessToken(): Promise<Result<string, AuthError>>;

  // 認証状態チェック
  isAuthenticated(): Promise<Result<boolean, AuthError>>;

  // ログイン（インタラクティブ）
  login(): Promise<Result<string, AuthError>>;

  // ログアウト
  logout(): Promise<Result<void, AuthError>>;

  // トークン無効化
  revokeToken(): Promise<Result<void, AuthError>>;
}
```

**使用例**:

```typescript
const authManager = getAuthManager();

// トークン取得
const result = await authManager.getAccessToken();
if (result.ok) {
  console.log('Token:', result.value);
} else {
  console.error('Error:', result.error.type, result.error.message);
}
```

### SyncManager

文字起こしの同期処理を管理。

```typescript
interface ISyncManager {
  // 同期開始
  startSync(documentId: string): Promise<Result<void, SyncError>>;

  // 同期停止
  stopSync(): Promise<Result<void, SyncError>>;

  // 文字起こし処理
  processTranscription(message: TranscriptionMessage): Promise<Result<void, SyncError>>;

  // 現在の状態取得
  getState(): SyncState;
}
```

**状態遷移**:

```
idle → syncing → idle
  ↓       ↓
error ← queued ← offline
  ↓
idle (retry後)
```

### GoogleDocsClient

Google Docs APIとの通信を担当。

```typescript
interface IGoogleDocsClient {
  // ドキュメント取得
  getDocument(documentId: string): Promise<Result<Document, ApiError>>;

  // テキスト挿入
  insertText(
    documentId: string,
    text: string,
    index: number
  ): Promise<Result<void, ApiError>>;

  // Named Range作成
  createNamedRange(
    documentId: string,
    name: string,
    startIndex: number,
    endIndex: number
  ): Promise<Result<string, ApiError>>;

  // Named Range削除
  deleteNamedRange(documentId: string, rangeId: string): Promise<Result<void, ApiError>>;
}
```

### QueueManager

オフライン時のメッセージキュー管理。

```typescript
interface IQueueManager {
  // キューに追加
  enqueue(item: QueueItem): Promise<Result<void, StorageError>>;

  // キューから取得（FIFO）
  dequeue(): Promise<Result<QueueItem | null, StorageError>>;

  // キューサイズ
  size(): Promise<Result<number, StorageError>>;

  // キュー全体取得
  getAll(): Promise<Result<QueueItem[], StorageError>>;

  // クリア
  clear(): Promise<Result<void, StorageError>>;
}
```

---

## テスト

### ユニットテスト

```bash
cd chrome-extension
npm test                    # 全テスト実行
npm run test:coverage       # カバレッジレポート
npm test -- tests/auth/     # 認証モジュールのみ
```

### E2Eテスト

```bash
npm run test:e2e           # Playwright E2E（headed mode）
npm run test:e2e:debug     # デバッグモード
```

**注意**: E2EテストはChrome拡張をロードするため、headedモードが必須。

### パフォーマンステスト

```bash
npm test -- tests/performance/
```

### テストカバレッジ目標

| モジュール | 目標 |
|------------|------|
| AuthManager | 90%+ |
| SyncManager | 85%+ |
| GoogleDocsClient | 80%+ |
| QueueManager | 90%+ |

---

## トラブルシューティング

### 認証エラー

**症状**: `chrome.identity.getAuthToken` が失敗

**解決策**:
1. manifest.jsonの`oauth2.client_id`を確認
2. Google Cloud Consoleで拡張機能IDが正しいか確認
3. `chrome://extensions` で拡張機能を再読み込み

### API呼び出しエラー

**症状**: 403 Forbidden

**解決策**:
1. Google Cloud ConsoleでDocsAPIが有効か確認
2. スコープが正しいか確認
3. ドキュメントへのアクセス権限を確認

### WebSocket接続エラー

**症状**: Tauriアプリとの接続が確立しない

**解決策**:
1. Tauriアプリが起動しているか確認
2. ポート9001-9100が使用可能か確認
3. `chrome.storage.local`の`ws_cached_port`をクリア

### オフラインキューの問題

**症状**: キューが処理されない

**解決策**:
1. `chrome.storage.local`の使用量を確認（上限5MB）
2. NetworkMonitorの状態を確認
3. Background Service Workerのログを確認

---

## コントリビューションガイド

### ブランチ戦略

- `main`: リリース用
- `develop`: 開発統合用
- `feature/*`: 新機能
- `fix/*`: バグ修正

### コミットメッセージ

```
feat(docs-sync): REQ-XXX 機能追加
fix(auth): バグ修正
test(sync): テスト追加
docs: ドキュメント更新
```

### プルリクエスト

1. `develop`ブランチからフィーチャーブランチを作成
2. 変更を実装
3. テストを追加・実行
4. PRを作成（テンプレートに従う）
5. レビュー後マージ

### コーディング規約

- TypeScript strict mode
- ESLint + Prettier
- Result<T, E>パターンでエラーハンドリング
- 日本語コメント可（ただしAPI名は英語）

---

*Last Updated: 2025-12-30*
