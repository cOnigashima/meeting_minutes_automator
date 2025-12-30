/**
 * トークンリフレッシュロジックインターフェース
 *
 * 責務: リフレッシュトークンを使用した新しいアクセストークン取得
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 *
 * Reference: design-artifacts/interface-contracts.md
 */

import type { Result } from '@/types/Result';
import type { RefreshError } from '@/types/AuthTypes';
import type { AlarmError } from '@/types/ChromeTypes';

export interface ITokenRefresher {
  /**
   * リフレッシュトークンを使用してアクセストークンを更新
   *
   * @preconditions refreshToken が有効
   * @postconditions 新しいアクセストークンが返される
   * @throws RefreshError リフレッシュトークンが無効、または一時的な失敗
   */
  refreshAccessToken(refreshToken: string): Promise<Result<string, RefreshError>>;

  /**
   * トークン有効期限の監視を開始（chrome.alarms使用）
   *
   * @preconditions expiresAt が未来の日時
   * @postconditions 有効期限60秒前にアラームが設定される
   * @throws AlarmError アラーム設定失敗
   */
  startExpiryMonitor(expiresAt: number): Promise<Result<void, AlarmError>>;
}
