/**
 * chrome.alarms管理インターフェース
 *
 * 責務: トークン有効期限監視とアラーム管理
 * テスト戦略: Chrome API モック化必要（⭐⭐⭐）
 *
 * Reference: design-artifacts/interface-contracts.md
 */

import type { Result } from '@/types/Result';
import type { AlarmError } from '@/types/ChromeTypes';

export interface ITokenExpiryMonitor {
  /**
   * トークン有効期限監視アラームを作成
   *
   * @preconditions expiresAt が未来の日時
   * @postconditions chrome.alarmsにアラームが登録される
   * @throws AlarmError アラーム作成失敗
   */
  createAlarm(expiresAt: number): Promise<Result<void, AlarmError>>;

  /**
   * アラームを削除
   *
   * @postconditions chrome.alarmsからアラームが削除される
   * @throws AlarmError アラーム削除失敗
   */
  clearAlarm(): Promise<Result<void, AlarmError>>;
}
