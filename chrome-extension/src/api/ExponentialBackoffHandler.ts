/**
 * Exponential Backoffリトライ戦略実装
 *
 * 責務: リトライ可能エラー時の自動リトライ（指数バックオフ + Jitter）
 * ファイルパス: chrome-extension/src/api/ExponentialBackoffHandler.ts
 *
 * Implementation: Phase 2
 *
 * リトライ可能エラー: 408, 429, 500, 502, 503, 504
 * バックオフ: 初回1秒、最大60秒、Jitter付き
 * 最大リトライ: 5回
 */

import { ok, err, type Result } from '../types/Result';
import type { MaxRetriesExceededError } from '../types/ApiTypes';
import type { IExponentialBackoffHandler } from './IExponentialBackoffHandler';

/**
 * リトライ可能なエラーを表すカスタムエラー
 */
export class RetryableError extends Error {
  constructor(
    message: string,
    public readonly statusCode: number
  ) {
    super(message);
    this.name = 'RetryableError';
  }
}

export class ExponentialBackoffHandler implements IExponentialBackoffHandler {
  private readonly maxRetries: number;
  private readonly baseDelayMs: number;
  private readonly maxDelayMs: number;

  constructor(
    maxRetries = 5,
    baseDelayMs = 1000,
    maxDelayMs = 60000
  ) {
    this.maxRetries = maxRetries;
    this.baseDelayMs = baseDelayMs;
    this.maxDelayMs = maxDelayMs;
  }

  /**
   * 指数バックオフ付きでfnを実行
   *
   * @param fn 実行する関数。RetryableErrorをthrowするとリトライ
   * @returns 成功時はResult.ok、最大リトライ超過時はMaxRetriesExceededError
   */
  async executeWithBackoff<T>(
    fn: () => Promise<T>
  ): Promise<Result<T, MaxRetriesExceededError>> {
    let lastError: Error | null = null;

    for (let attempt = 0; attempt <= this.maxRetries; attempt++) {
      try {
        const result = await fn();
        return ok(result);
      } catch (error) {
        if (!(error instanceof RetryableError)) {
          // リトライ不可能なエラーはそのままthrow
          throw error;
        }

        lastError = error;

        if (attempt < this.maxRetries) {
          const delayMs = this.calculateDelay(attempt);
          await this.sleep(delayMs);
        }
      }
    }

    return err({
      type: 'MaxRetriesExceeded',
      message: lastError?.message || 'Max retries exceeded',
      retriesAttempted: this.maxRetries,
    });
  }

  /**
   * 指数バックオフ遅延を計算（Jitter付き）
   *
   * 計算式: min(baseDelay * 2^attempt + jitter, maxDelay)
   */
  private calculateDelay(attempt: number): number {
    const exponentialDelay = this.baseDelayMs * Math.pow(2, attempt);
    const jitter = Math.random() * this.baseDelayMs; // 0〜baseDelayのランダム
    return Math.min(exponentialDelay + jitter, this.maxDelayMs);
  }

  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  /**
   * HTTPステータスコードがリトライ可能かどうか判定
   */
  static isRetryableStatus(statusCode: number): boolean {
    return [408, 429, 500, 502, 503, 504].includes(statusCode);
  }
}
