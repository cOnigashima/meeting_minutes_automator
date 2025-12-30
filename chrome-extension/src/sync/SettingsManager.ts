/**
 * Settings Manager
 *
 * Google Docs同期設定の永続化と取得を管理する。
 * chrome.storage.localを使用して設定を保存。
 *
 * Implementation: Phase 5 - Task 13.1
 */

import type { Result } from '@/types/Result';
import type { DocsSyncSettings } from '@/types/SyncTypes';
import type { StorageWriteError } from '@/types/ChromeTypes';
import { ok, err } from '@/types/Result';
import { DEFAULT_DOCS_SYNC_SETTINGS } from '@/types/SyncTypes';

const SETTINGS_KEY = 'docs_sync_settings';

export interface ISettingsManager {
  getSettings(): Promise<Result<DocsSyncSettings, StorageWriteError>>;
  saveSettings(settings: DocsSyncSettings): Promise<Result<void, StorageWriteError>>;
  updateSettings(partial: Partial<DocsSyncSettings>): Promise<Result<DocsSyncSettings, StorageWriteError>>;
  resetToDefaults(): Promise<Result<DocsSyncSettings, StorageWriteError>>;
}

export class SettingsManager implements ISettingsManager {
  /**
   * 現在の設定を取得する
   * 設定が存在しない場合はデフォルト値を返す
   */
  async getSettings(): Promise<Result<DocsSyncSettings, StorageWriteError>> {
    try {
      const result = await chrome.storage.local.get(SETTINGS_KEY);
      const stored = result[SETTINGS_KEY] as Partial<DocsSyncSettings> | undefined;

      // Merge with defaults to ensure all fields exist
      const settings: DocsSyncSettings = {
        ...DEFAULT_DOCS_SYNC_SETTINGS,
        ...stored,
      };

      return ok(settings);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      return err({
        type: 'StorageWriteFailed',
        message,
      });
    }
  }

  /**
   * 設定を保存する
   */
  async saveSettings(settings: DocsSyncSettings): Promise<Result<void, StorageWriteError>> {
    try {
      // Validate bufferingSeconds range
      const validated: DocsSyncSettings = {
        ...settings,
        bufferingSeconds: Math.min(5, Math.max(1, settings.bufferingSeconds)),
      };

      await chrome.storage.local.set({ [SETTINGS_KEY]: validated });
      return ok(undefined);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      return err({
        type: message.includes('QUOTA_BYTES') ? 'StorageQuotaExceeded' : 'StorageWriteFailed',
        message,
      });
    }
  }

  /**
   * 設定を部分的に更新する
   */
  async updateSettings(partial: Partial<DocsSyncSettings>): Promise<Result<DocsSyncSettings, StorageWriteError>> {
    const currentResult = await this.getSettings();
    if (!currentResult.ok) {
      return currentResult;
    }

    const updated: DocsSyncSettings = {
      ...currentResult.value,
      ...partial,
    };

    const saveResult = await this.saveSettings(updated);
    if (!saveResult.ok) {
      return saveResult;
    }

    return ok(updated);
  }

  /**
   * デフォルト設定にリセットする
   */
  async resetToDefaults(): Promise<Result<DocsSyncSettings, StorageWriteError>> {
    const saveResult = await this.saveSettings(DEFAULT_DOCS_SYNC_SETTINGS);
    if (!saveResult.ok) {
      return saveResult;
    }

    return ok(DEFAULT_DOCS_SYNC_SETTINGS);
  }
}

// Singleton instance
let settingsManagerInstance: SettingsManager | null = null;

export function getSettingsManager(): SettingsManager {
  if (!settingsManagerInstance) {
    settingsManagerInstance = new SettingsManager();
  }
  return settingsManagerInstance;
}
