/**
 * 認証コンポーネントファクトリー
 *
 * 責務: 認証関連コンポーネントの組み立てと依存性注入
 * ファイルパス: chrome-extension/src/auth/AuthFactory.ts
 *
 * Implementation: Phase 1 (getAuthToken版)
 *
 * Note: getAuthToken() を使用するため、構成が大幅に簡素化:
 * - TokenStore 不要（Chromeがキャッシュ管理）
 * - TokenRefresher 不要（Chromeが有効期限管理）
 * - TokenExpiryMonitor 不要（Chromeが自動更新）
 */

import { AuthManager } from './AuthManager';
import { ChromeIdentityClient } from './ChromeIdentityClient';
import type { IAuthManager } from './IAuthManager';

/**
 * AuthManager のシングルトンインスタンスを生成
 *
 * コンポーネント構成:
 * - ChromeIdentityClient: chrome.identity.getAuthToken()
 */
export function createAuthManager(): IAuthManager {
  const identityClient = new ChromeIdentityClient();
  return new AuthManager(identityClient);
}

// シングルトンインスタンス（遅延初期化）
let authManagerInstance: IAuthManager | null = null;

/**
 * AuthManager のシングルトンインスタンスを取得
 */
export function getAuthManager(): IAuthManager {
  if (!authManagerInstance) {
    authManagerInstance = createAuthManager();
  }
  return authManagerInstance;
}
