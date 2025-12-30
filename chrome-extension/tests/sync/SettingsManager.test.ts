/**
 * SettingsManager Tests
 *
 * Implementation: Phase 5 - Task 13.1
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { SettingsManager } from '../../src/sync/SettingsManager';
import { DEFAULT_DOCS_SYNC_SETTINGS } from '../../src/types/SyncTypes';

// Mock chrome.storage.local
const mockStorage: Record<string, unknown> = {};

vi.stubGlobal('chrome', {
  storage: {
    local: {
      get: vi.fn((keys: string | string[]) => {
        return Promise.resolve(
          typeof keys === 'string'
            ? { [keys]: mockStorage[keys] }
            : keys.reduce((acc, key) => ({ ...acc, [key]: mockStorage[key] }), {})
        );
      }),
      set: vi.fn((items: Record<string, unknown>) => {
        Object.assign(mockStorage, items);
        return Promise.resolve();
      }),
      remove: vi.fn((keys: string | string[]) => {
        const keysArray = typeof keys === 'string' ? [keys] : keys;
        keysArray.forEach((key) => delete mockStorage[key]);
        return Promise.resolve();
      }),
    },
  },
});

describe('SettingsManager', () => {
  let settingsManager: SettingsManager;

  beforeEach(() => {
    // Clear mock storage
    Object.keys(mockStorage).forEach((key) => delete mockStorage[key]);
    vi.clearAllMocks();
    settingsManager = new SettingsManager();
  });

  describe('getSettings', () => {
    it('should return default settings when no settings are stored', async () => {
      const result = await settingsManager.getSettings();

      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value).toEqual(DEFAULT_DOCS_SYNC_SETTINGS);
      }
    });

    it('should return stored settings merged with defaults', async () => {
      mockStorage['docs_sync_settings'] = { enabled: false };

      const result = await settingsManager.getSettings();

      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value.enabled).toBe(false);
        expect(result.value.showTimestamp).toBe(true); // From defaults
        expect(result.value.showSpeaker).toBe(false); // From defaults
        expect(result.value.bufferingSeconds).toBe(3); // From defaults
      }
    });
  });

  describe('saveSettings', () => {
    it('should save settings to storage', async () => {
      const settings = {
        enabled: false,
        showTimestamp: false,
        showSpeaker: true,
        bufferingSeconds: 5,
      };

      const result = await settingsManager.saveSettings(settings);

      expect(result.ok).toBe(true);
      expect(mockStorage['docs_sync_settings']).toEqual(settings);
    });

    it('should validate bufferingSeconds range (clamp to 1-5)', async () => {
      const settings = {
        enabled: true,
        showTimestamp: true,
        showSpeaker: false,
        bufferingSeconds: 10, // Over max
      };

      await settingsManager.saveSettings(settings);

      expect(mockStorage['docs_sync_settings']).toEqual({
        ...settings,
        bufferingSeconds: 5, // Clamped to max
      });
    });

    it('should clamp bufferingSeconds to minimum of 1', async () => {
      const settings = {
        enabled: true,
        showTimestamp: true,
        showSpeaker: false,
        bufferingSeconds: 0, // Under min
      };

      await settingsManager.saveSettings(settings);

      expect(mockStorage['docs_sync_settings']).toEqual({
        ...settings,
        bufferingSeconds: 1, // Clamped to min
      });
    });
  });

  describe('updateSettings', () => {
    it('should update only specified settings', async () => {
      // Start with defaults
      const updateResult = await settingsManager.updateSettings({ enabled: false });

      expect(updateResult.ok).toBe(true);
      if (updateResult.ok) {
        expect(updateResult.value.enabled).toBe(false);
        expect(updateResult.value.showTimestamp).toBe(true); // Unchanged
      }
    });

    it('should update multiple settings at once', async () => {
      const updateResult = await settingsManager.updateSettings({
        enabled: false,
        showSpeaker: true,
        bufferingSeconds: 2,
      });

      expect(updateResult.ok).toBe(true);
      if (updateResult.ok) {
        expect(updateResult.value.enabled).toBe(false);
        expect(updateResult.value.showSpeaker).toBe(true);
        expect(updateResult.value.bufferingSeconds).toBe(2);
        expect(updateResult.value.showTimestamp).toBe(true); // Unchanged
      }
    });
  });

  describe('resetToDefaults', () => {
    it('should reset all settings to defaults', async () => {
      // First, save custom settings
      await settingsManager.saveSettings({
        enabled: false,
        showTimestamp: false,
        showSpeaker: true,
        bufferingSeconds: 1,
      });

      // Then reset
      const result = await settingsManager.resetToDefaults();

      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value).toEqual(DEFAULT_DOCS_SYNC_SETTINGS);
      }
    });
  });
});
