/**
 * オフラインキュー操作インターフェース
 *
 * 責務: オフライン時のメッセージキューイング、FIFO順で送信
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */

import type { Result } from '@/types/Result';
import type { TranscriptionMessage, StorageFullError } from '@/types/SyncTypes';

import type { StorageWriteError } from '@/types/ChromeTypes';

export interface IQueueManager {
  enqueue(message: TranscriptionMessage): Promise<Result<void, StorageFullError>>;
  dequeueAll(): Promise<Result<TranscriptionMessage[], StorageWriteError>>;
  peekAll(): Promise<Result<TranscriptionMessage[], StorageWriteError>>;
  removeFirst(count: number): Promise<Result<void, StorageWriteError>>;
  clear(): Promise<Result<void, StorageWriteError>>;
  size(): Promise<Result<number, StorageWriteError>>;
}
