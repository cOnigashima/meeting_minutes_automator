import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { ChromeIdentityClient } from '../../src/auth/ChromeIdentityClient';

// Mock chrome.identity
const mockIdentity = {
  getAuthToken: vi.fn(),
  removeCachedAuthToken: vi.fn(),
};

// Mock chrome.runtime
const mockRuntime = {
  lastError: null as { message?: string } | null,
};

vi.stubGlobal('chrome', {
  identity: mockIdentity,
  runtime: mockRuntime,
});

describe('ChromeIdentityClient', () => {
  let client: ChromeIdentityClient;

  beforeEach(() => {
    client = new ChromeIdentityClient();
    vi.clearAllMocks();
    mockRuntime.lastError = null;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('getAccessToken', () => {
    it('should call chrome.identity.getAuthToken with interactive=true', async () => {
      mockIdentity.getAuthToken.mockImplementation(
        (options: { interactive: boolean }, callback: (token?: string) => void) => {
          callback('test-access-token');
        }
      );

      const result = await client.getAccessToken(true);

      expect(mockIdentity.getAuthToken).toHaveBeenCalledWith(
        { interactive: true },
        expect.any(Function)
      );
      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value).toBe('test-access-token');
      }
    });

    it('should call chrome.identity.getAuthToken with interactive=false', async () => {
      mockIdentity.getAuthToken.mockImplementation(
        (options: { interactive: boolean }, callback: (token?: string) => void) => {
          callback('cached-access-token');
        }
      );

      const result = await client.getAccessToken(false);

      expect(mockIdentity.getAuthToken).toHaveBeenCalledWith(
        { interactive: false },
        expect.any(Function)
      );
      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value).toBe('cached-access-token');
      }
    });

    it('should return UserCancelledError when user cancels', async () => {
      mockIdentity.getAuthToken.mockImplementation(
        (options: { interactive: boolean }, callback: (token?: string) => void) => {
          mockRuntime.lastError = { message: 'The user did not approve access' };
          callback(undefined);
        }
      );

      const result = await client.getAccessToken(true);

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('UserCancelled');
      }
    });

    it('should return NetworkError when getAuthToken fails', async () => {
      mockIdentity.getAuthToken.mockImplementation(
        (options: { interactive: boolean }, callback: (token?: string) => void) => {
          mockRuntime.lastError = { message: 'OAuth2 not granted or revoked' };
          callback(undefined);
        }
      );

      const result = await client.getAccessToken(false);

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('NetworkError');
        expect(result.error.message).toContain('OAuth2 not granted');
      }
    });

    it('should return NetworkError when no token returned', async () => {
      mockIdentity.getAuthToken.mockImplementation(
        (options: { interactive: boolean }, callback: (token?: string) => void) => {
          callback(undefined);
        }
      );

      const result = await client.getAccessToken(true);

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('NetworkError');
        expect(result.error.message).toBe('No token returned');
      }
    });
  });

  describe('removeCachedToken', () => {
    it('should call chrome.identity.removeCachedAuthToken', async () => {
      mockIdentity.removeCachedAuthToken.mockImplementation(
        (options: { token: string }, callback: () => void) => {
          callback();
        }
      );

      await client.removeCachedToken('test-token-to-remove');

      expect(mockIdentity.removeCachedAuthToken).toHaveBeenCalledWith(
        { token: 'test-token-to-remove' },
        expect.any(Function)
      );
    });
  });
});
