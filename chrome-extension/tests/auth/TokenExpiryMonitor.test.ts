import { describe, it, expect, vi, beforeEach } from 'vitest';
import { TokenExpiryMonitor } from '../../src/auth/TokenExpiryMonitor';

// Mock chrome.alarms
const mockChromeAlarms = {
  create: vi.fn(async () => {}),
  clear: vi.fn(async () => true),
};

vi.stubGlobal('chrome', { alarms: mockChromeAlarms });

describe('TokenExpiryMonitor', () => {
  let monitor: TokenExpiryMonitor;

  beforeEach(() => {
    monitor = new TokenExpiryMonitor();
    vi.clearAllMocks();
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2025-01-01T00:00:00Z'));
  });

  describe('createAlarm', () => {
    it('should call chrome.alarms.create with correct delay', async () => {
      const now = Date.now();
      const expiresAt = now + 3600 * 1000; // 1 hour from now

      const result = await monitor.createAlarm(expiresAt);

      expect(result.ok).toBe(true);
      expect(mockChromeAlarms.clear).toHaveBeenCalledWith('token_expiry_alarm');
      expect(mockChromeAlarms.create).toHaveBeenCalledWith('token_expiry_alarm', {
        when: expect.any(Number),
      });
    });

    it('should convert expiresAt to alarm delay (60 seconds before expiry)', async () => {
      const now = Date.now();
      const expiresAt = now + 3600 * 1000; // 1 hour from now
      const expectedAlarmTime = expiresAt - 60 * 1000; // 60 seconds before expiry

      await monitor.createAlarm(expiresAt);

      const createCall = mockChromeAlarms.create.mock.calls[0];
      expect(createCall[1].when).toBe(expectedAlarmTime);
    });

    it('should use fixed alarm name (token_expiry_alarm)', async () => {
      const expiresAt = Date.now() + 3600 * 1000;

      await monitor.createAlarm(expiresAt);

      expect(mockChromeAlarms.create).toHaveBeenCalledWith(
        'token_expiry_alarm',
        expect.any(Object)
      );
    });

    it('should use minimum 1 second delay for nearly expired tokens', async () => {
      const now = Date.now();
      const expiresAt = now + 30 * 1000; // 30 seconds from now (less than PRE_REFRESH_SECONDS)

      await monitor.createAlarm(expiresAt);

      const createCall = mockChromeAlarms.create.mock.calls[0];
      // Should use at least 1 second delay
      expect(createCall[1].when).toBeGreaterThanOrEqual(now + 1000);
    });
  });

  describe('clearAlarm', () => {
    it('should call chrome.alarms.clear', async () => {
      const result = await monitor.clearAlarm();

      expect(result.ok).toBe(true);
      expect(mockChromeAlarms.clear).toHaveBeenCalledWith('token_expiry_alarm');
    });

    it('should succeed even if alarm does not exist', async () => {
      // chrome.alarms.clear returns true even if alarm doesn't exist
      mockChromeAlarms.clear.mockResolvedValueOnce(false);

      const result = await monitor.clearAlarm();

      expect(result.ok).toBe(true);
    });
  });

  describe('getAlarmName', () => {
    it('should return the fixed alarm name', () => {
      expect(monitor.getAlarmName()).toBe('token_expiry_alarm');
    });
  });
});
