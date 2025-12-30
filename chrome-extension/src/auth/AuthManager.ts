/**
 * OAuth 2.0認証フロー統合実装
 *
 * 責務: 認証フロー実行とトークン取得
 * ファイルパス: chrome-extension/src/auth/AuthManager.ts
 *
 * Implementation: Phase 1 (getAuthToken版)
 *
 * Note: chrome.identity.getAuthToken() を使用することで:
 * - トークンキャッシュ・有効期限はChrome管理
 * - refresh_token の手動管理不要
 * - TokenRefresher 不要
 */

import { ok, err, type Result } from '../types/Result';
import type { AuthTokens, AuthError, TokenExpiredError, RevokeError } from '../types/AuthTypes';
import type { IAuthManager } from './IAuthManager';
import type { IChromeIdentityClient } from './IChromeIdentityClient';

export class AuthManager implements IAuthManager {
  private cachedToken: string | null = null;

  constructor(private identityClient: IChromeIdentityClient) {}

  /**
   * OAuth 2.0認証フローを開始し、トークンを取得
   *
   * interactive: true で同意画面を表示
   */
  async initiateAuth(): Promise<Result<AuthTokens, AuthError>> {
    const result = await this.identityClient.getAccessToken(true);

    if (!result.ok) {
      const error = result.error;
      if (error.type === 'UserCancelled') {
        return err({ type: 'UserCancelled' });
      }
      return err({ type: 'NetworkError', message: error.message });
    }

    this.cachedToken = result.value;

    // getAuthToken() はトークンの有効期限をChrome側で管理するため、
    // expiresAt は概算値として1時間後を設定
    const tokens: AuthTokens = {
      accessToken: result.value,
      refreshToken: '', // getAuthToken()ではChrome管理のため不要
      expiresAt: Date.now() + 3600 * 1000,
    };

    return ok(tokens);
  }

  /**
   * アクセストークンを取得
   *
   * Chromeがキャッシュと有効期限を管理するため、
   * まずinteractive: falseで試行し、失敗したらエラーを返す
   */
  async getAccessToken(): Promise<Result<string, TokenExpiredError>> {
    // まずnon-interactiveで試行
    const result = await this.identityClient.getAccessToken(false);

    if (result.ok) {
      this.cachedToken = result.value;
      return ok(result.value);
    }

    // non-interactiveで失敗した場合、再認証が必要
    return err({
      type: 'RefreshRequired',
      message: result.error.message,
    });
  }

  /**
   * トークンを無効化
   */
  async revokeToken(): Promise<Result<void, RevokeError>> {
    try {
      if (this.cachedToken) {
        // Chromeのキャッシュからトークンを削除
        await this.identityClient.removeCachedToken(this.cachedToken);

        // Google OAuth revoke endpoint を呼び出し（ベストエフォート）
        try {
          await fetch(`https://oauth2.googleapis.com/revoke?token=${this.cachedToken}`, {
            method: 'POST',
          });
        } catch {
          // revoke失敗は無視（ベストエフォート）
        }

        this.cachedToken = null;
      }

      return ok(undefined);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      return err({ type: 'RevokeFailed', message });
    }
  }

  /**
   * 認証状態を取得
   *
   * non-interactiveでトークン取得を試み、成功したら認証済み
   */
  async isAuthenticated(): Promise<boolean> {
    const result = await this.identityClient.getAccessToken(false);
    if (result.ok) {
      this.cachedToken = result.value;
      return true;
    }
    return false;
  }
}
