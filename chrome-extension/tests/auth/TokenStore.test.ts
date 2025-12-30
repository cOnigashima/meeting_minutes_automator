import { describe, it, expect, vi, beforeEach } from 'vitest';
import { TokenStore } from '../../src/auth/TokenStore';
import type { AuthTokens } from '../../src/types/AuthTypes';

// Mock chrome.storage.local
const mockStorage: Record<string, unknown> = {};

const mockChromeStorage = {
  local: {
    get: vi.fn(async (key: string) => {
      return { [key]: mockStorage[key] };
    }),
    set: vi.fn(async (items: Record<string, unknown>) => {
      Object.assign(mockStorage, items);
    }),
    remove: vi.fn(async (key: string) => {
      delete mockStorage[key];
    }),
  },
};

vi.stubGlobal('chrome', { storage: mockChromeStorage });

describe('TokenStore', () => {
  let tokenStore: TokenStore;

  const sampleTokens: AuthTokens = {
    accessToken: 'test-access-token',
    refreshToken: 'test-refresh-token',
    expiresAt: Date.now() + 3600 * 1000,
  };

  beforeEach(() => {
    tokenStore = new TokenStore();
    // Clear mock storage
    Object.keys(mockStorage).forEach(key => delete mockStorage[key]);
    vi.clearAllMocks();
  });

  describe('save', () => {
    it('should save tokens to chrome.storage.local', async () => {
      const result = await tokenStore.save(sampleTokens);

      expect(result.ok).toBe(true);
      expect(mockChromeStorage.local.set).toHaveBeenCalledWith({
        auth_tokens: sampleTokens,
      });
    });

    it('should return StorageError when quota exceeded', async () => {
      mockChromeStorage.local.set.mockRejectedValueOnce(
        new Error('QUOTA_BYTES quota exceeded')
      );

      const result = await tokenStore.save(sampleTokens);

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('QuotaExceeded');
      }
    });

    it('should overwrite existing tokens', async () => {
      // Save first tokens
      await tokenStore.save(sampleTokens);

      const newTokens: AuthTokens = {
        accessToken: 'new-access-token',
        refreshToken: 'new-refresh-token',
        expiresAt: Date.now() + 7200 * 1000,
      };

      // Overwrite with new tokens
      const result = await tokenStore.save(newTokens);

      expect(result.ok).toBe(true);
      expect(mockChromeStorage.local.set).toHaveBeenLastCalledWith({
        auth_tokens: newTokens,
      });
    });
  });

  describe('load', () => {
    it('should return tokens from chrome.storage.local', async () => {
      mockStorage['auth_tokens'] = sampleTokens;

      const result = await tokenStore.load();

      expect(result).toEqual(sampleTokens);
    });

    it('should return null when no tokens exist', async () => {
      const result = await tokenStore.load();

      expect(result).toBeNull();
    });

    it('should parse stored JSON correctly', async () => {
      const storedTokens: AuthTokens = {
        accessToken: 'stored-token',
        refreshToken: 'stored-refresh',
        expiresAt: 1234567890000,
      };
      mockStorage['auth_tokens'] = storedTokens;

      const result = await tokenStore.load();

      expect(result).not.toBeNull();
      expect(result?.accessToken).toBe('stored-token');
      expect(result?.refreshToken).toBe('stored-refresh');
      expect(result?.expiresAt).toBe(1234567890000);
    });
  });

  describe('remove', () => {
    it('should delete tokens from chrome.storage.local', async () => {
      mockStorage['auth_tokens'] = sampleTokens;

      await tokenStore.remove();

      expect(mockChromeStorage.local.remove).toHaveBeenCalledWith('auth_tokens');
    });

    it('should succeed even if no tokens exist', async () => {
      // No tokens in storage
      await expect(tokenStore.remove()).resolves.not.toThrow();
      expect(mockChromeStorage.local.remove).toHaveBeenCalledWith('auth_tokens');
    });
  });
});
