/**
 * レート制限制御実装
 *
 * Implementation: Phase 3
 */

import type { ITokenBucketRateLimiter } from './ITokenBucketRateLimiter';

export class TokenBucketRateLimiter implements ITokenBucketRateLimiter {
  private readonly capacity = 60; // 60 requests per minute
  private tokens = 60;
  private lastRefillAt = Date.now();

  async acquire(): Promise<void> {
    while (true) {
      this.refillTokens();
      if (this.tokens > 0) {
        this.tokens -= 1;
        return;
      }

      const now = Date.now();
      const waitMs = Math.max(0, 1000 - (now - this.lastRefillAt));
      await this.sleep(waitMs);
    }
  }

  getAvailableTokens(): number {
    this.refillTokens();
    return this.tokens;
  }

  private refillTokens(): void {
    const now = Date.now();
    const elapsedSeconds = Math.floor((now - this.lastRefillAt) / 1000);
    if (elapsedSeconds <= 0) return;
    this.tokens = Math.min(this.capacity, this.tokens + elapsedSeconds);
    this.lastRefillAt += elapsedSeconds * 1000;
  }

  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}
