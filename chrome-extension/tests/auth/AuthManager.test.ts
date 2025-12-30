import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { AuthManager } from '../../src/auth/AuthManager';
import type { IChromeIdentityClient } from '../../src/auth/IChromeIdentityClient';
import { ok, err } from '../../src/types/Result';

// Mock fetch for revokeToken
const mockFetch = vi.fn();
vi.stubGlobal('fetch', mockFetch);

describe('AuthManager', () => {
  let authManager: AuthManager;
  let mockIdentityClient: IChromeIdentityClient;

  beforeEach(() => {
    mockIdentityClient = {
      getAccessToken: vi.fn(),
      removeCachedToken: vi.fn().mockResolvedValue(undefined),
    };

    authManager = new AuthManager(mockIdentityClient);
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('initiateAuth', () => {
    it('should call getAccessToken with interactive=true', async () => {
      vi.mocked(mockIdentityClient.getAccessToken).mockResolvedValue(ok('access-token-123'));

      await authManager.initiateAuth();

      expect(mockIdentityClient.getAccessToken).toHaveBeenCalledWith(true);
    });

    it('should return AuthTokens when auth succeeds', async () => {
      vi.mocked(mockIdentityClient.getAccessToken).mockResolvedValue(ok('access-token-xyz'));

      const result = await authManager.initiateAuth();

      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value.accessToken).toBe('access-token-xyz');
        expect(result.value.refreshToken).toBe(''); // Not used with getAuthToken
        expect(result.value.expiresAt).toBeGreaterThan(Date.now());
      }
    });

    it('should return UserCancelledError when user cancels', async () => {
      vi.mocked(mockIdentityClient.getAccessToken).mockResolvedValue(
        err({ type: 'UserCancelled' as const, message: 'User closed the window' })
      );

      const result = await authManager.initiateAuth();

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('UserCancelled');
      }
    });

    it('should return NetworkError when network fails', async () => {
      vi.mocked(mockIdentityClient.getAccessToken).mockResolvedValue(
        err({ type: 'NetworkError' as const, message: 'Network error' })
      );

      const result = await authManager.initiateAuth();

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('NetworkError');
      }
    });
  });

  describe('getAccessToken', () => {
    it('should return access token when getAuthToken succeeds', async () => {
      vi.mocked(mockIdentityClient.getAccessToken).mockResolvedValue(ok('valid-access-token'));

      const result = await authManager.getAccessToken();

      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value).toBe('valid-access-token');
      }
      expect(mockIdentityClient.getAccessToken).toHaveBeenCalledWith(false);
    });

    it('should return RefreshRequired when non-interactive getAuthToken fails', async () => {
      vi.mocked(mockIdentityClient.getAccessToken).mockResolvedValue(
        err({ type: 'NetworkError' as const, message: 'OAuth2 not granted' })
      );

      const result = await authManager.getAccessToken();

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('RefreshRequired');
      }
    });
  });

  describe('revokeToken', () => {
    it('should remove cached token and call revoke endpoint', async () => {
      // First, authenticate to cache a token
      vi.mocked(mockIdentityClient.getAccessToken).mockResolvedValue(ok('token-to-revoke'));
      await authManager.initiateAuth();

      mockFetch.mockResolvedValueOnce({ ok: true });

      const result = await authManager.revokeToken();

      expect(result.ok).toBe(true);
      expect(mockIdentityClient.removeCachedToken).toHaveBeenCalledWith('token-to-revoke');
      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('https://oauth2.googleapis.com/revoke'),
        expect.objectContaining({ method: 'POST' })
      );
    });

    it('should return success even if revoke endpoint fails (best effort)', async () => {
      // First, authenticate to cache a token
      vi.mocked(mockIdentityClient.getAccessToken).mockResolvedValue(ok('token-to-revoke'));
      await authManager.initiateAuth();

      mockFetch.mockRejectedValueOnce(new Error('Network error'));

      const result = await authManager.revokeToken();

      expect(result.ok).toBe(true);
      expect(mockIdentityClient.removeCachedToken).toHaveBeenCalled();
    });

    it('should do nothing if no token is cached', async () => {
      const result = await authManager.revokeToken();

      expect(result.ok).toBe(true);
      expect(mockIdentityClient.removeCachedToken).not.toHaveBeenCalled();
      expect(mockFetch).not.toHaveBeenCalled();
    });
  });

  describe('isAuthenticated', () => {
    it('should return true when non-interactive getAuthToken succeeds', async () => {
      vi.mocked(mockIdentityClient.getAccessToken).mockResolvedValue(ok('valid-token'));

      const result = await authManager.isAuthenticated();

      expect(result).toBe(true);
      expect(mockIdentityClient.getAccessToken).toHaveBeenCalledWith(false);
    });

    it('should return false when non-interactive getAuthToken fails', async () => {
      vi.mocked(mockIdentityClient.getAccessToken).mockResolvedValue(
        err({ type: 'NetworkError' as const, message: 'No token' })
      );

      const result = await authManager.isAuthenticated();

      expect(result).toBe(false);
    });
  });
});
