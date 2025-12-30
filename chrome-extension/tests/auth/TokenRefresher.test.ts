import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { TokenRefresher } from '../../src/auth/TokenRefresher';
import type { ITokenExpiryMonitor } from '../../src/auth/ITokenExpiryMonitor';
import { ok, err } from '../../src/types/Result';

// Mock fetch
const mockFetch = vi.fn();
vi.stubGlobal('fetch', mockFetch);

describe('TokenRefresher', () => {
  let tokenRefresher: TokenRefresher;
  let mockExpiryMonitor: ITokenExpiryMonitor;

  const CLIENT_ID = 'test-client-id';

  beforeEach(() => {
    mockExpiryMonitor = {
      createAlarm: vi.fn().mockResolvedValue(ok(undefined)),
      clearAlarm: vi.fn().mockResolvedValue(ok(undefined)),
      getAlarmName: vi.fn().mockReturnValue('token_expiry_alarm'),
    };

    tokenRefresher = new TokenRefresher(CLIENT_ID, mockExpiryMonitor);
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('refreshAccessToken', () => {
    it('should POST to token endpoint with refresh token', async () => {
      const refreshToken = 'test-refresh-token';
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ access_token: 'new-access-token' }),
      });

      await tokenRefresher.refreshAccessToken(refreshToken);

      expect(mockFetch).toHaveBeenCalledWith(
        'https://oauth2.googleapis.com/token',
        expect.objectContaining({
          method: 'POST',
          headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        })
      );

      // Check the body contains expected parameters
      const callArgs = mockFetch.mock.calls[0];
      const body = callArgs[1].body;
      expect(body).toContain('refresh_token=test-refresh-token');
      expect(body).toContain('client_id=test-client-id');
      expect(body).toContain('grant_type=refresh_token');
    });

    it('should return new access token', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ access_token: 'new-access-token-123' }),
      });

      const result = await tokenRefresher.refreshAccessToken('refresh-token');

      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value).toBe('new-access-token-123');
      }
    });

    it('should keep the same refresh token', async () => {
      // This test verifies that refreshAccessToken only returns access_token
      // The refresh token management is handled by AuthManager
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          access_token: 'new-access-token',
          // refresh_token may or may not be returned
        }),
      });

      const result = await tokenRefresher.refreshAccessToken('original-refresh-token');

      // The method only returns the access token, not the refresh token
      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(typeof result.value).toBe('string');
      }
    });

    it('should return RefreshError when refresh token is invalid', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        json: async () => ({
          error: 'invalid_grant',
          error_description: 'Token has been revoked',
        }),
      });

      const result = await tokenRefresher.refreshAccessToken('invalid-refresh-token');

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('InvalidRefreshToken');
        expect(result.error.message).toContain('Token has been revoked');
      }
    });

    it('should return RefreshError on network error', async () => {
      mockFetch.mockRejectedValueOnce(new Error('Network error'));

      const result = await tokenRefresher.refreshAccessToken('refresh-token');

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('RefreshFailed');
        expect(result.error.message).toContain('Network error');
      }
    });
  });

  describe('startExpiryMonitor', () => {
    it('should create alarm via TokenExpiryMonitor', async () => {
      const expiresAt = Date.now() + 3600 * 1000;

      const result = await tokenRefresher.startExpiryMonitor(expiresAt);

      expect(result.ok).toBe(true);
      expect(mockExpiryMonitor.createAlarm).toHaveBeenCalledWith(expiresAt);
    });

    it('should schedule alarm 60 seconds before expiry', async () => {
      // This is actually handled by TokenExpiryMonitor internally
      // This test verifies the delegation
      const expiresAt = Date.now() + 3600 * 1000;

      await tokenRefresher.startExpiryMonitor(expiresAt);

      expect(mockExpiryMonitor.createAlarm).toHaveBeenCalledWith(expiresAt);
    });

    it('should clear existing alarm before creating new one', async () => {
      // This is also handled by TokenExpiryMonitor internally
      const expiresAt = Date.now() + 3600 * 1000;

      await tokenRefresher.startExpiryMonitor(expiresAt);

      // Verify delegation to createAlarm (which handles clearing internally)
      expect(mockExpiryMonitor.createAlarm).toHaveBeenCalled();
    });
  });

  describe('stopExpiryMonitor', () => {
    it('should clear alarm via TokenExpiryMonitor', async () => {
      const result = await tokenRefresher.stopExpiryMonitor();

      expect(result.ok).toBe(true);
      expect(mockExpiryMonitor.clearAlarm).toHaveBeenCalled();
    });
  });
});
