/**
 * Chrome Identity API の実装
 *
 * 責務: chrome.identity.getAuthToken() によるトークン取得
 * ファイルパス: chrome-extension/src/auth/ChromeIdentityClient.ts
 *
 * Implementation: Phase 1 (getAuthToken版)
 *
 * Note: getAuthToken() を使用することで:
 * - client_secret 不要
 * - redirect_uri 登録不要
 * - トークンキャッシュ・有効期限はChrome管理
 */

import { ok, err, type Result } from '../types/Result';
import type { AuthFlowError } from '../types/AuthTypes';
import type { IChromeIdentityClient } from './IChromeIdentityClient';

export class ChromeIdentityClient implements IChromeIdentityClient {
  /**
   * アクセストークンを取得する
   *
   * @param interactive trueの場合、同意画面を表示可能
   */
  async getAccessToken(interactive: boolean): Promise<Result<string, AuthFlowError>> {
    return new Promise((resolve) => {
      chrome.identity.getAuthToken({ interactive }, (token) => {
        const lastError = chrome.runtime.lastError;

        if (lastError) {
          const message = lastError.message || 'Unknown error';

          // ユーザーキャンセル判定
          if (
            message.includes('canceled') ||
            message.includes('cancelled') ||
            message.includes('user did not approve')
          ) {
            resolve(err({ type: 'UserCancelled', message }));
            return;
          }

          // その他のエラー
          resolve(err({ type: 'NetworkError', message }));
          return;
        }

        if (!token) {
          resolve(err({ type: 'NetworkError', message: 'No token returned' }));
          return;
        }

        resolve(ok(token));
      });
    });
  }

  /**
   * キャッシュされたトークンを削除する
   *
   * 401エラー時などにトークンを無効化してから再取得するために使用
   */
  async removeCachedToken(token: string): Promise<void> {
    return new Promise((resolve) => {
      chrome.identity.removeCachedAuthToken({ token }, () => {
        resolve();
      });
    });
  }
}
