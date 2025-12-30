import { describe, it, expect, vi, afterEach } from 'vitest';
import { ExponentialBackoffHandler, RetryableError } from '@/api/ExponentialBackoffHandler';

describe('ExponentialBackoffHandler', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('executeWithBackoff', () => {
    it('should return immediately on first success', async () => {
      const handler = new ExponentialBackoffHandler(3, 1, 10);
      const sleepSpy = vi
        .spyOn(handler as unknown as { sleep: (ms: number) => Promise<void> }, 'sleep')
        .mockResolvedValue();
      const fn = vi.fn().mockResolvedValue('ok');

      const result = await handler.executeWithBackoff(fn);

      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value).toBe('ok');
      }
      expect(fn).toHaveBeenCalledTimes(1);
      expect(sleepSpy).not.toHaveBeenCalled();
    });

    it('should retry on RetryableError and succeed', async () => {
      const handler = new ExponentialBackoffHandler(3, 1, 10);
      const sleepSpy = vi
        .spyOn(handler as unknown as { sleep: (ms: number) => Promise<void> }, 'sleep')
        .mockResolvedValue();
      let attempts = 0;
      const fn = vi.fn(async () => {
        if (attempts === 0) {
          attempts += 1;
          throw new RetryableError('retry', 429);
        }
        return 'ok';
      });

      const result = await handler.executeWithBackoff(fn);

      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value).toBe('ok');
      }
      expect(fn).toHaveBeenCalledTimes(2);
      expect(sleepSpy).toHaveBeenCalledTimes(1);
    });

    it('should return MaxRetriesExceededError after max retries', async () => {
      const handler = new ExponentialBackoffHandler(2, 1, 10);
      const sleepSpy = vi
        .spyOn(handler as unknown as { sleep: (ms: number) => Promise<void> }, 'sleep')
        .mockResolvedValue();
      const fn = vi.fn(async () => {
        throw new RetryableError('retry', 503);
      });

      const result = await handler.executeWithBackoff(fn);

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('MaxRetriesExceeded');
        expect(result.error.retriesAttempted).toBe(2);
      }
      expect(fn).toHaveBeenCalledTimes(3);
      expect(sleepSpy).toHaveBeenCalledTimes(2);
    });

    it('should throw non-retryable errors', async () => {
      const handler = new ExponentialBackoffHandler(3, 1, 10);
      const fn = vi.fn(async () => {
        throw new Error('boom');
      });

      await expect(handler.executeWithBackoff(fn)).rejects.toThrow('boom');
      expect(fn).toHaveBeenCalledTimes(1);
    });
  });

  describe('isRetryableStatus', () => {
    it('should return true for retryable HTTP status codes', () => {
      expect(ExponentialBackoffHandler.isRetryableStatus(408)).toBe(true);
      expect(ExponentialBackoffHandler.isRetryableStatus(429)).toBe(true);
      expect(ExponentialBackoffHandler.isRetryableStatus(500)).toBe(true);
      expect(ExponentialBackoffHandler.isRetryableStatus(502)).toBe(true);
      expect(ExponentialBackoffHandler.isRetryableStatus(503)).toBe(true);
      expect(ExponentialBackoffHandler.isRetryableStatus(504)).toBe(true);
    });

    it('should return false for non-retryable HTTP status codes', () => {
      expect(ExponentialBackoffHandler.isRetryableStatus(400)).toBe(false);
      expect(ExponentialBackoffHandler.isRetryableStatus(401)).toBe(false);
      expect(ExponentialBackoffHandler.isRetryableStatus(404)).toBe(false);
    });
  });
});
