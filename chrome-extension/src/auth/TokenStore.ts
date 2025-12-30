/**
 * トークンストレージの実装
 *
 * 責務: トークンの永続化のみ（検証ロジックは含まない）
 * ファイルパス: chrome-extension/src/auth/TokenStore.ts
 *
 * Implementation: Phase 1
 */

import { ok, err, type Result } from '../types/Result';
import type { AuthTokens, StorageError } from '../types/AuthTypes';
import type { ITokenStore } from './ITokenStore';

export class TokenStore implements ITokenStore {
  private readonly STORAGE_KEY = 'auth_tokens';

  /**
   * トークンを chrome.storage.local に保存する
   */
  async save(token: AuthTokens): Promise<Result<void, StorageError>> {
    try {
      await chrome.storage.local.set({ [this.STORAGE_KEY]: token });
      return ok(undefined);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      // QuotaExceededError の判定
      if (message.includes('QUOTA_BYTES')) {
        return err({ type: 'QuotaExceeded', message });
      }
      return err({ type: 'WriteError', message });
    }
  }

  /**
   * chrome.storage.local からトークンを読み込む
   */
  async load(): Promise<AuthTokens | null> {
    const result = await chrome.storage.local.get(this.STORAGE_KEY);
    const tokens = result[this.STORAGE_KEY] as AuthTokens | undefined;
    return tokens ?? null;
  }

  /**
   * chrome.storage.local からトークンを削除する
   */
  async remove(): Promise<void> {
    await chrome.storage.local.remove(this.STORAGE_KEY);
  }
}
