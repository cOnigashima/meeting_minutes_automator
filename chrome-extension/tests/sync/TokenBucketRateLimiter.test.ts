import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { TokenBucketRateLimiter } from '@/sync/TokenBucketRateLimiter';

describe('TokenBucketRateLimiter', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(0);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  describe('acquire', () => {
    it('should resolve immediately when tokens available', async () => {
      const limiter = new TokenBucketRateLimiter();

      await limiter.acquire();

      expect(limiter.getAvailableTokens()).toBe(59);
    });

    it('should wait when no tokens available', async () => {
      const limiter = new TokenBucketRateLimiter();

      for (let i = 0; i < 60; i += 1) {
        await limiter.acquire();
      }

      let resolved = false;
      const pending = limiter.acquire().then(() => {
        resolved = true;
      });

      await vi.advanceTimersByTimeAsync(999);
      expect(resolved).toBe(false);

      await vi.advanceTimersByTimeAsync(1);
      await pending;
      expect(resolved).toBe(true);
      expect(limiter.getAvailableTokens()).toBe(0);
    });

    it('should refill tokens at 1 token per second', async () => {
      const limiter = new TokenBucketRateLimiter();

      for (let i = 0; i < 60; i += 1) {
        await limiter.acquire();
      }

      vi.advanceTimersByTime(3000);
      expect(limiter.getAvailableTokens()).toBe(3);
    });

    it('should enforce 60 tokens max capacity', async () => {
      const limiter = new TokenBucketRateLimiter();

      vi.advanceTimersByTime(120000);
      expect(limiter.getAvailableTokens()).toBe(60);
    });
  });

  describe('getAvailableTokens', () => {
    it('should return current token count', async () => {
      const limiter = new TokenBucketRateLimiter();

      await limiter.acquire();

      expect(limiter.getAvailableTokens()).toBe(59);
    });
  });
});
