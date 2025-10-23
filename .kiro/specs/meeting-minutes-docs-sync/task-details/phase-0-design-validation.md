# Phase 0: Design Validation & Skeleton Implementation (Week 0) ⚙️ UPDATED

> **親ドキュメント**: [tasks.md](../tasks.md) | [task-details/README.md](README.md)
> **関連設計**: [design-components.md](../design-modules/design-components.md) | [design-architecture.md](../design-modules/design-architecture.md)
> **Requirements**: 全要件（設計検証フェーズ）

## Goal

実装前の詳細設計検証とTDD環境整備。**Critical Issuesを解決し、実装可能なスケルトンを生成**。クラス図・責務マトリクス・インターフェース契約定義を完成させ、19クラス全てのスケルトン実装とテストスケルトンを生成し、TDD実装の基盤を確立。

**重要**: Phase 0は「設計図作成」ではなく「実装可能性の検証」が目的。最もリスクの高いOAuth 2.0 + Google Docs API統合を先に検証（Vertical Slice Spike）してから、19クラスへの分割を行う。

---

## 🚨 Critical Issues (Phase 0開始前に必須対応)

以下の4つのCritical Issuesが未解決のため、現状では実装不可能：

1. **ファイルパス不整合**: 設計では`extension/src/...`だが、実際は`chrome-extension/src/...`
2. **テストインフラ未整備**: Vitest/Jestが存在せず、`it.todo()`も実行不可能
3. **tsconfig path alias未設定**: `@/auth/AuthManager`等のインポートがコンパイルエラー
4. **Phase 0完了判定矛盾**: クラス図は完成済みだが、インターフェース契約がスケルトン版

→ **Task 0.1で最優先対応**

---

## Task Overview (全10タスク、7日間)

| Task | Focus | Day | Status |
|------|-------|-----|--------|
| 0.1 | テストインフラ整備（Vitest + path alias） | Day 1 | 未着手 |
| 0.2 | インターフェース契約定義の完成 | Day 1-2 | 未着手 |
| 0.3 | クラス図の最終レビュー（chrome.alarms修正確認） | Day 2 | 未着手 |
| 0.4 | **Vertical Slice Spike** (OAuth+Docs統合検証) | Day 3-4 | 未着手 |
| 0.5 | Auth Domainスケルトン実装（5クラス） | Day 5 | 未着手 |
| 0.6 | Sync Domainスケルトン実装（8クラス） | Day 5 | 未着手 |
| 0.7 | API Domainスケルトン実装（6クラス） | Day 5 | 未着手 |
| 0.8 | 全19クラスのテストスケルトン生成 | Day 6 | 未着手 |
| 0.9 | 設計検証チェックリスト実行 | Day 7 | 未着手 |
| 0.10 | Phase 0成果物レビュー | Day 7 | 未着手 |

---

## 0.1 テストインフラ整備（CRITICAL - 最優先）

**目的**: TDD実装に必須のテストフレームワーク・path alias・実行スクリプトを整備

**受け入れ基準**:
- [ ] Vitest導入: `chrome-extension/package.json`に`vitest`と`@vitest/ui`を追加
- [ ] tsconfig path alias設定: `"@/*": ["./src/*"]`を`chrome-extension/tsconfig.json`に追加
- [ ] テスト実行スクリプト追加: `"test": "vitest"`, `"test:ui": "vitest --ui"`
- [ ] サンプルテスト作成: `chrome-extension/tests/sample.test.ts`（動作確認用）
- [ ] テスト実行成功確認: `cd chrome-extension && npm test`

**技術詳細**:
```json
// chrome-extension/package.json (追加部分)
{
  "scripts": {
    "test": "vitest",
    "test:ui": "vitest --ui",
    "test:coverage": "vitest --coverage"
  },
  "devDependencies": {
    "vitest": "^1.0.0",
    "@vitest/ui": "^1.0.0",
    "@vitest/coverage-v8": "^1.0.0"
  }
}
```

```json
// chrome-extension/tsconfig.json (追加部分)
{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["./src/*"]
    }
  }
}
```

```typescript
// chrome-extension/tests/sample.test.ts (サンプル)
import { describe, it, expect } from 'vitest';

describe('Sample Test', () => {
  it('should pass', () => {
    expect(1 + 1).toBe(2);
  });
});
```

**依存**: なし（最優先タスク）

---

## 0.2 インターフェース契約定義の完成

**目的**: 残り17インターフェースの事前条件/事後条件/エラー型を完成させる

**受け入れ基準**:
- [ ] 全19インターフェースの完全な契約定義（現状: 2/19完成）
- [ ] 全メソッドに`@preconditions`/`@postconditions`/`@throws`記載
- [ ] Result<T, E>型の一貫性確認
- [ ] コード例を含むドキュメント更新

**現状**: `design-artifacts/interface-contracts.md`にAuth Domainの2インターフェース（IChromeIdentityClient, ITokenStore）のみ完成

**対応**: 残り17インターフェースを同じフォーマットで追加
- Auth Domain: 3インターフェース（ITokenRefresher, ITokenExpiryMonitor, IAuthManager）
- Sync Domain: 8インターフェース
- API Domain: 6インターフェース

**成果物**: `design-artifacts/interface-contracts.md`（完全版）

**依存**: Task 0.1（path alias設定後、型インポートが正常動作）

---

## 0.3 クラス図の最終レビュー

**目的**: 既存クラス図のレビューとchrome.alarms修正の反映確認

**受け入れ基準**:
- [ ] auth-domain.md: ファイルパス修正（`extension/` → `chrome-extension/`）
- [ ] sync-domain.md: `chrome.alarms` → `setInterval (Offscreen Document)`修正確認
  - BufferingManager: `ALARM_NAME` → `INTERVAL_ID`
  - StorageMonitor: `ALARM_NAME` → `INTERVAL_ID`
- [ ] api-domain.md: ファイルパス修正
- [ ] 循環依存チェック（全19クラス）

**現状**: クラス図は既に完成済み（auth/sync/api-domain.md）

**対応**: ファイルパス修正 + chrome.alarms対応確認のみ

**成果物**: 修正後のクラス図3ファイル

**依存**: Task 0.2完了後

---

## 0.4 Vertical Slice Spike (OAuth+Docs統合検証) - 新規追加 🆕

**目的**: 最もリスクの高いOAuth 2.0 + Google Docs API統合をMV3環境で先に検証

**背景**:
- 19クラス全部のスケルトンを作る前に、**最難関部分が動くことを証明**する
- OAuth 2.0 + Manifest V3 Service Workerの組み合わせは実装リスクが高い
- Spikeで検証後、動作確認済みのコードを19クラスに分割する方が安全

**受け入れ基準**:
- [ ] 最小限のAuthManager実装: `chrome.identity.launchWebAuthFlow()`のみ
- [ ] 最小限のGoogleDocsClient実装: `documents.batchUpdate`でテキスト挿入のみ
- [ ] E2Eスパイクテスト: `auth-docs-integration.test.ts`
- [ ] 実際のGoogleアカウントでOAuth 2.0認証成功
- [ ] アクセストークン取得確認
- [ ] Google Docs APIへの1回のテキスト挿入成功
- [ ] MV3 Service Worker環境での動作確認

**実装ファイル**:
```
chrome-extension/src/
├── spike/
│   ├── MinimalAuthManager.ts (最小実装)
│   └── MinimalGoogleDocsClient.ts (最小実装)
└── tests/
    └── spike/
        └── auth-docs-integration.test.ts (E2Eテスト)
```

**スパイクコード例**:
```typescript
// chrome-extension/src/spike/MinimalAuthManager.ts
export class MinimalAuthManager {
  async getAccessToken(): Promise<string> {
    // chrome.identity.launchWebAuthFlow() の最小実装
    const redirectUrl = chrome.identity.getRedirectURL();
    const authUrl = `https://accounts.google.com/o/oauth2/auth?...`;

    const responseUrl = await chrome.identity.launchWebAuthFlow({
      url: authUrl,
      interactive: true
    });

    // アクセストークンをURLから抽出
    const token = new URL(responseUrl).searchParams.get('access_token');
    return token;
  }
}
```

```typescript
// chrome-extension/src/spike/MinimalGoogleDocsClient.ts
export class MinimalGoogleDocsClient {
  async insertText(documentId: string, text: string, accessToken: string): Promise<void> {
    const response = await fetch(
      `https://docs.googleapis.com/v1/documents/${documentId}:batchUpdate`,
      {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${accessToken}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          requests: [{
            insertText: {
              text: text,
              location: { index: 1 }
            }
          }]
        })
      }
    );

    if (!response.ok) {
      throw new Error(`API Error: ${response.status}`);
    }
  }
}
```

**成功基準**:
- OAuth 2.0フロー成功率 > 95%
- Google Docs API呼び出し成功率 > 95%
- Service Workerタイムアウトなし

**失敗時の対応**:
- Phase 0を中断し、設計を見直す（例: Tauri側でOAuth処理、拡張はIPC経由のみ）

**依存**: Task 0.1（テストインフラ整備完了後）

---

## 0.5 Auth Domainスケルトン実装（5クラス）

**目的**: Spikeで検証したAuthManagerを5クラスに分割し、スケルトン実装を生成

**受け入れ基準**:
- [ ] `chrome-extension/src/auth/AuthManager.ts`
- [ ] `chrome-extension/src/auth/ChromeIdentityClient.ts`
- [ ] `chrome-extension/src/auth/TokenStore.ts`
- [ ] `chrome-extension/src/auth/TokenRefresher.ts`
- [ ] `chrome-extension/src/auth/TokenExpiryMonitor.ts`
- [ ] 全メソッドに`throw new Error('Not implemented')`
- [ ] 依存性注入パターンの一貫性
- [ ] インターフェース実装の完全性（interface-contracts.mdに準拠）
- [ ] TypeScriptコンパイル成功（`npm run build`）

**スケルトン例**:
```typescript
// chrome-extension/src/auth/AuthManager.ts

import { IAuthManager, IChromeIdentityClient, ITokenStore, ITokenRefresher } from './interfaces';
import { AuthToken, AuthError } from './types';
import { Result } from '@/types/Result';

export class AuthManager implements IAuthManager {
  constructor(
    private authClient: IChromeIdentityClient,
    private tokenStore: ITokenStore,
    private tokenRefresher: ITokenRefresher
  ) {}

  async initiateAuth(): Promise<Result<AuthToken, AuthError>> {
    // TODO: Phase 1, Task 1.1 - OAuth 2.0認証フロー実装
    // Spike実装 (MinimalAuthManager.ts) を参考に実装
    throw new Error('Not implemented');
  }

  async refreshToken(refreshToken: string): Promise<Result<AuthToken, AuthError>> {
    // TODO: Phase 1, Task 1.3 - トークンリフレッシュ実装
    throw new Error('Not implemented');
  }

  async revokeToken(accessToken: string): Promise<void> {
    // TODO: Phase 1, Task 1.1 - トークン無効化実装
    throw new Error('Not implemented');
  }
}
```

**ファイルパス**: `chrome-extension/src/auth/` (NOT `extension/src/auth/`)

**依存**: Task 0.4（Spike成功後）

---

## 0.6 Sync Domainスケルトン実装（8クラス）

**目的**: Sync Domain全8クラスのスケルトン実装を生成

**受け入れ基準**:
- [ ] `chrome-extension/src/sync/SyncManager.ts`
- [ ] `chrome-extension/src/sync/SyncStateMachine.ts`
- [ ] `chrome-extension/src/sync/QueueManager.ts`
- [ ] `chrome-extension/src/sync/BufferingManager.ts` (setInterval使用)
- [ ] `chrome-extension/src/sync/TokenBucketRateLimiter.ts`
- [ ] `chrome-extension/src/sync/NetworkMonitor.ts`
- [ ] `chrome-extension/src/sync/StorageMonitor.ts` (setInterval使用)
- [ ] `chrome-extension/src/sync/ResyncOrchestrator.ts`
- [ ] TypeScriptコンパイル成功

**重要**: BufferingManagerとStorageMonitorは`chrome.alarms`ではなく、Offscreen Document上の`setInterval`を使用（Task 6.2, 7.2で修正済み）

**依存**: Task 0.5完了後

---

## 0.7 API Domainスケルトン実装（6クラス）

**目的**: Spikeで検証したGoogleDocsClientを6クラスに分割し、スケルトン実装を生成

**受け入れ基準**:
- [ ] `chrome-extension/src/api/GoogleDocsClient.ts`
- [ ] `chrome-extension/src/api/ExponentialBackoffHandler.ts`
- [ ] `chrome-extension/src/api/OptimisticLockHandler.ts`
- [ ] `chrome-extension/src/api/NamedRangeManager.ts`
- [ ] `chrome-extension/src/api/NamedRangeRecoveryStrategy.ts`
- [ ] `chrome-extension/src/api/ParagraphStyleFormatter.ts`
- [ ] TypeScriptコンパイル成功

**依存**: Task 0.6完了後

---

## 0.8 全19クラスのテストスケルトン生成

**目的**: 全19クラスのテストファイル生成（`it.todo()`列挙）

**受け入れ基準**:
- [ ] `chrome-extension/tests/auth/` (5ファイル)
- [ ] `chrome-extension/tests/sync/` (8ファイル)
- [ ] `chrome-extension/tests/api/` (6ファイル)
- [ ] 各テストファイルに`describe`/`it.todo`構造
- [ ] テストケース列挙（正常系/異常系/境界値）
- [ ] テスト実行成功（`npm test` → 全テストがtodo状態）

**テストスケルトン例**:
```typescript
// chrome-extension/tests/auth/AuthManager.test.ts

import { describe, it, expect, beforeEach } from 'vitest';
import { AuthManager } from '@/auth/AuthManager';
import { MockChromeIdentityClient } from '../mocks/MockChromeIdentityClient';
import { MockTokenStore } from '../mocks/MockTokenStore';
import { MockTokenRefresher } from '../mocks/MockTokenRefresher';

describe('AuthManager', () => {
  let authManager: AuthManager;
  let mockAuthClient: MockChromeIdentityClient;
  let mockTokenStore: MockTokenStore;
  let mockTokenRefresher: MockTokenRefresher;

  beforeEach(() => {
    mockAuthClient = new MockChromeIdentityClient();
    mockTokenStore = new MockTokenStore();
    mockTokenRefresher = new MockTokenRefresher();
    authManager = new AuthManager(mockAuthClient, mockTokenStore, mockTokenRefresher);
  });

  describe('initiateAuth()', () => {
    it.todo('should launch auth flow and save token');
    it.todo('should handle user cancellation');
    it.todo('should handle network error');
    it.todo('should handle invalid grant error');
    it.todo('should schedule token refresh after successful auth');
  });

  describe('refreshToken()', () => {
    it.todo('should refresh token with valid refresh token');
    it.todo('should handle invalid refresh token');
    it.todo('should update token store after refresh');
    it.todo('should reschedule next refresh');
  });

  describe('revokeToken()', () => {
    it.todo('should revoke token on Google OAuth server');
    it.todo('should remove token from local storage');
    it.todo('should handle network error during revocation');
  });
});
```

**テスト実行結果例**:
```
$ npm test

 ✓ chrome-extension/tests/sample.test.ts (1)
 ⚠ chrome-extension/tests/auth/AuthManager.test.ts (11 todos)
 ⚠ chrome-extension/tests/sync/SyncManager.test.ts (9 todos)
 ...

Test Files  20 passed (20)
     Tests  1 passed | 150 todos (151)
```

**依存**: Task 0.7完了後

---

## 0.9 設計検証チェックリスト実行

**目的**: SOLID原則・責務分割・テスト容易性の最終確認

**受け入れ基準**:
- [ ] 各クラスが単一責務を持つ（SRP）
- [ ] 依存関係が一方向（循環依存なし）
- [ ] 公開メソッド数が5個以下（全19クラス）
- [ ] プライベートメソッド数が2個以下（全19クラス）
- [ ] テスト容易性⭐4以上が80%以上（18/19クラス）
- [ ] 全インターフェースに事前条件/事後条件記載
- [ ] Result<T, E>型の一貫性確認

**検証ツール**:
- TypeScript Compiler: 型エラー0件
- ESLint: 循環依存チェック（`import/no-cycle`）
- 責務マトリクス: メトリクス確認

**依存**: Task 0.8完了後

---

## 0.10 Phase 0成果物レビュー

**目的**: Phase 0完了判定とPhase 1移行条件確認

**受け入れ基準**:
- [ ] 全19クラスのクラス図承認
- [ ] 責務マトリクス承認（テスト容易性⭐4以上が80%以上）
- [ ] インターフェース契約承認（全19インターフェース完成）
- [ ] スケルトン実装生成完了（TypeScriptコンパイル成功）
- [ ] テストスケルトン生成完了（`npm test`で150+ todos表示）
- [ ] Vertical Slice Spike成功（OAuth+Docs統合動作確認）

**Phase 1移行条件**:
- [ ] クラス図に循環依存なし
- [ ] テスト容易性⭐4以上が80%以上
- [ ] 全インターフェースに契約定義あり
- [ ] スケルトン実装が全てコンパイル成功
- [ ] **Spike成功（OAuth 2.0 + Google Docs API動作確認済み）**

**依存**: Task 0.9完了後

---

## Progress Tracking

**Group A: 環境セットアップ（Day 1）**
- [ ] Task 0.1: テストインフラ整備（Vitest + path alias）

**Group B: 設計成果物完成（Day 1-2）**
- [ ] Task 0.2: インターフェース契約定義の完成（17インターフェース追加）
- [ ] Task 0.3: クラス図の最終レビュー（ファイルパス修正 + chrome.alarms確認）

**Group C: Vertical Slice Spike（Day 3-4）**
- [ ] Task 0.4: OAuth+Docs統合検証（最難関部分の事前検証）

**Group D: 19クラススケルトン生成（Day 5）**
- [ ] Task 0.5: Auth Domainスケルトン実装（5クラス）
- [ ] Task 0.6: Sync Domainスケルトン実装（8クラス）
- [ ] Task 0.7: API Domainスケルトン実装（6クラス）

**Group E: テストスケルトン生成（Day 6）**
- [ ] Task 0.8: 全19クラスのテストスケルトン生成（150+ todos）

**Group F: 設計検証（Day 7）**
- [ ] Task 0.9: 設計検証チェックリスト実行
- [ ] Task 0.10: Phase 0成果物レビュー

**Total**: 0/10 tasks completed

---

## Critical Issues解決状況

| Issue | 解決策 | 対応タスク |
|-------|--------|-----------|
| ✅ ファイルパス不整合 | 全タスクで`chrome-extension/src/`に統一 | Task 0.3, 0.5-0.7 |
| ✅ テストインフラ未整備 | Vitest + path alias設定 | Task 0.1 |
| ✅ tsconfig path alias未設定 | `@/*` → `./src/*`追加 | Task 0.1 |
| ✅ Phase 0完了判定矛盾 | 明確な受け入れ基準設定 | Task 0.10 |
| ✅ 実装リスク（OAuth+Docs） | Vertical Slice Spikeで事前検証 | Task 0.4 |

---

## References

- [design-components.md](../design-modules/design-components.md): 既存コンポーネント設計
- [design-architecture.md](../design-modules/design-architecture.md): アーキテクチャ概要
- [design-testing-security.md](../design-modules/design-testing-security.md): テスト戦略
- [Auth Domain Class Diagram](/docs/uml/meeting-minutes-docs-sync/cls/auth-domain.md): 完成済みクラス図（5クラス）
- [Sync Domain Class Diagram](/docs/uml/meeting-minutes-docs-sync/cls/sync-domain.md): 完成済みクラス図（8クラス）
- [API Domain Class Diagram](/docs/uml/meeting-minutes-docs-sync/cls/api-domain.md): 完成済みクラス図（6クラス）
- [design-artifacts/responsibility-matrix.md](../design-artifacts/responsibility-matrix.md): 完成済み責務マトリクス
- [design-artifacts/interface-contracts.md](../design-artifacts/interface-contracts.md): インターフェース契約（要完成）
- [SOLID Principles](https://en.wikipedia.org/wiki/SOLID): 単一責務原則等
- [Vitest Documentation](https://vitest.dev/): テストフレームワーク

---

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-10-24 | 1.0 | Claude Code | 初版作成（16タスク構成） |
| 2025-10-24 | 2.0 | Claude Code | Critical Issues対応版（10タスクに再構成、Vertical Slice Spike追加、ファイルパス修正） |
