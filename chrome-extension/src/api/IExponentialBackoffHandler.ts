/**
 * Exponential Backoffリトライ戦略インターフェース
 *
 * 責務: 429 Too Many Requests時のリトライロジック
 * テスト戦略: 完全にモック可能（⭐⭐⭐⭐⭐）
 */

import type { Result } from '@/types/Result';
import type { MaxRetriesExceededError } from '@/types/ApiTypes';

export interface IExponentialBackoffHandler {
  executeWithBackoff<T>(
    fn: () => Promise<T>
  ): Promise<Result<T, MaxRetriesExceededError>>;
}
