/**
 * chrome.alarms管理実装
 *
 * 責務: トークン有効期限監視とアラーム管理
 * ファイルパス: chrome-extension/src/auth/TokenExpiryMonitor.ts
 *
 * Implementation: Phase 1
 */

import { ok, err, type Result } from '../types/Result';
import type { AlarmError } from '../types/ChromeTypes';
import type { ITokenExpiryMonitor } from './ITokenExpiryMonitor';

export class TokenExpiryMonitor implements ITokenExpiryMonitor {
  private readonly ALARM_NAME = 'token_expiry_alarm';
  // 有効期限の60秒前にアラームを発火（クロックスキュー対策）
  private readonly PRE_REFRESH_SECONDS = 60;

  /**
   * トークン有効期限監視アラームを作成
   *
   * 有効期限の60秒前にアラームが発火する
   */
  async createAlarm(expiresAt: number): Promise<Result<void, AlarmError>> {
    try {
      // 既存のアラームをクリア
      await chrome.alarms.clear(this.ALARM_NAME);

      const now = Date.now();
      const alarmTime = expiresAt - this.PRE_REFRESH_SECONDS * 1000;

      // 既に期限切れまたは間もなく期限切れの場合は即座にアラーム
      const delayMs = Math.max(alarmTime - now, 1000); // 最低1秒後

      await chrome.alarms.create(this.ALARM_NAME, {
        when: now + delayMs,
      });

      return ok(undefined);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      return err({ type: 'AlarmCreateFailed', message });
    }
  }

  /**
   * アラームを削除
   */
  async clearAlarm(): Promise<Result<void, AlarmError>> {
    try {
      await chrome.alarms.clear(this.ALARM_NAME);
      return ok(undefined);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      return err({ type: 'AlarmClearFailed', message });
    }
  }

  /**
   * アラーム名を取得（テストやリスナー登録用）
   */
  getAlarmName(): string {
    return this.ALARM_NAME;
  }
}
