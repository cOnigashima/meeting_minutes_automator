/**
 * トークンリフレッシュロジック実装
 *
 * 責務: リフレッシュトークンを使用した新しいアクセストークン取得
 * ファイルパス: chrome-extension/src/auth/TokenRefresher.ts
 *
 * Implementation: Phase 1
 *
 * Note: 本番環境では Chrome App OAuth type を使用するため、
 * client_secret は不要（PKCE のみで認証可能）
 */

import { ok, err, type Result } from '../types/Result';
import type { RefreshError } from '../types/AuthTypes';
import type { AlarmError } from '../types/ChromeTypes';
import type { ITokenRefresher } from './ITokenRefresher';
import type { ITokenExpiryMonitor } from './ITokenExpiryMonitor';

export class TokenRefresher implements ITokenRefresher {
  private readonly clientId: string;

  constructor(
    clientId: string,
    private expiryMonitor: ITokenExpiryMonitor
  ) {
    this.clientId = clientId;
  }

  /**
   * リフレッシュトークンを使用してアクセストークンを更新
   *
   * Note: Chrome App OAuth type では client_secret 不要
   */
  async refreshAccessToken(refreshToken: string): Promise<Result<string, RefreshError>> {
    try {
      const tokenUrl = 'https://oauth2.googleapis.com/token';
      const body = new URLSearchParams({
        refresh_token: refreshToken,
        client_id: this.clientId,
        grant_type: 'refresh_token',
        // Note: client_secret は Chrome App OAuth type では不要
      });

      const response = await fetch(tokenUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        body: body.toString(),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        const errorCode = typeof errorData.error === 'string' ? errorData.error : '';
        const message =
          (typeof errorData.error_description === 'string' && errorData.error_description) ||
          errorCode ||
          `Token refresh failed (HTTP ${response.status})`;

        if (errorCode === 'invalid_grant') {
          return err({ type: 'InvalidRefreshToken', message });
        }

        return err({ type: 'RefreshFailed', message });
      }

      const data = await response.json();
      return ok(data.access_token);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      return err({ type: 'RefreshFailed', message });
    }
  }

  /**
   * トークン有効期限の監視を開始（chrome.alarms使用）
   *
   * 有効期限の60秒前にアラームが発火する
   */
  async startExpiryMonitor(expiresAt: number): Promise<Result<void, AlarmError>> {
    return this.expiryMonitor.createAlarm(expiresAt);
  }

  /**
   * 有効期限監視を停止
   */
  async stopExpiryMonitor(): Promise<Result<void, AlarmError>> {
    return this.expiryMonitor.clearAlarm();
  }
}
