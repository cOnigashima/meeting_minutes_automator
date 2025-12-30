/**
 * オフラインキュー操作実装
 *
 * Implementation: Phase 3
 */

import type { Result } from '@/types/Result';
import type { TranscriptionMessage, StorageFullError } from '@/types/SyncTypes';
import type { StorageWriteError } from '@/types/ChromeTypes';
import type { IQueueManager } from './IQueueManager';
import { ok, err } from '@/types/Result';

export class QueueManager implements IQueueManager {
  private readonly QUEUE_KEY = 'offline_queue';
  // NOTE: This queue should be written from a single extension context (ideally the service worker).
  // If other contexts need to enqueue, route through the service worker to avoid cross-context races.
  private static lock: Promise<void> = Promise.resolve();

  private async withLock<T>(fn: () => Promise<T>): Promise<T> {
    const run = QueueManager.lock.then(() => fn());
    QueueManager.lock = run.then(
      () => undefined,
      () => undefined
    );
    return run;
  }

  async enqueue(message: TranscriptionMessage): Promise<Result<void, StorageFullError>> {
    return this.withLock(async () => {
      try {
        const result = await chrome.storage.local.get(this.QUEUE_KEY);
        const queue = (result[this.QUEUE_KEY] as TranscriptionMessage[] | undefined) ?? [];
        queue.push(message);
        await chrome.storage.local.set({ [this.QUEUE_KEY]: queue });
        return ok(undefined);
      } catch (error) {
        const messageText = error instanceof Error ? error.message : String(error);
        return err({ type: 'QuotaExceeded', message: messageText });
      }
    });
  }

  async dequeueAll(): Promise<Result<TranscriptionMessage[], StorageWriteError>> {
    return this.withLock(async () => {
      try {
        const result = await chrome.storage.local.get(this.QUEUE_KEY);
        const queue = (result[this.QUEUE_KEY] as TranscriptionMessage[] | undefined) ?? [];
        await chrome.storage.local.remove(this.QUEUE_KEY);
        return ok(queue);
      } catch (error) {
        const messageText = error instanceof Error ? error.message : String(error);
        return err({
          type: messageText.includes('QUOTA_BYTES') ? 'StorageQuotaExceeded' : 'StorageWriteFailed',
          message: messageText,
        });
      }
    });
  }

  async peekAll(): Promise<Result<TranscriptionMessage[], StorageWriteError>> {
    return this.withLock(async () => {
      try {
        const result = await chrome.storage.local.get(this.QUEUE_KEY);
        const queue = (result[this.QUEUE_KEY] as TranscriptionMessage[] | undefined) ?? [];
        return ok(queue);
      } catch (error) {
        const messageText = error instanceof Error ? error.message : String(error);
        return err({
          type: messageText.includes('QUOTA_BYTES') ? 'StorageQuotaExceeded' : 'StorageWriteFailed',
          message: messageText,
        });
      }
    });
  }

  async removeFirst(count: number): Promise<Result<void, StorageWriteError>> {
    return this.withLock(async () => {
      try {
        const result = await chrome.storage.local.get(this.QUEUE_KEY);
        const queue = (result[this.QUEUE_KEY] as TranscriptionMessage[] | undefined) ?? [];
        const remaining = queue.slice(count);
        if (remaining.length === 0) {
          await chrome.storage.local.remove(this.QUEUE_KEY);
        } else {
          await chrome.storage.local.set({ [this.QUEUE_KEY]: remaining });
        }
        return ok(undefined);
      } catch (error) {
        const messageText = error instanceof Error ? error.message : String(error);
        return err({
          type: messageText.includes('QUOTA_BYTES') ? 'StorageQuotaExceeded' : 'StorageWriteFailed',
          message: messageText,
        });
      }
    });
  }

  async clear(): Promise<Result<void, StorageWriteError>> {
    return this.withLock(async () => {
      try {
        await chrome.storage.local.remove(this.QUEUE_KEY);
        return ok(undefined);
      } catch (error) {
        const messageText = error instanceof Error ? error.message : String(error);
        return err({
          type: messageText.includes('QUOTA_BYTES') ? 'StorageQuotaExceeded' : 'StorageWriteFailed',
          message: messageText,
        });
      }
    });
  }

  async size(): Promise<Result<number, StorageWriteError>> {
    return this.withLock(async () => {
      try {
        const result = await chrome.storage.local.get(this.QUEUE_KEY);
        const queue = (result[this.QUEUE_KEY] as TranscriptionMessage[] | undefined) ?? [];
        return ok(queue.length);
      } catch (error) {
        const messageText = error instanceof Error ? error.message : String(error);
        return err({
          type: messageText.includes('QUOTA_BYTES') ? 'StorageQuotaExceeded' : 'StorageWriteFailed',
          message: messageText,
        });
      }
    });
  }
}
