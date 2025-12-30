/**
 * 同期フロー統合実装
 *
 * Implementation: Phase 3
 */

import type { Result } from '@/types/Result';
import type { TranscriptionMessage, SyncStartError, ProcessError, ResyncError } from '@/types/SyncTypes';
import type { ApiError } from '@/types/ApiTypes';
import type { ISyncManager } from './ISyncManager';
import type { IGoogleDocsClient } from '@/api/IGoogleDocsClient';
import type { INamedRangeManager } from '@/api/INamedRangeManager';
import type { IQueueManager } from './IQueueManager';
import type { ITokenBucketRateLimiter } from './ITokenBucketRateLimiter';
import type { INetworkMonitor } from './INetworkMonitor';
import type { ISyncStateMachine } from './ISyncStateMachine';
import { ok, err } from '@/types/Result';

export class SyncManager implements ISyncManager {
  private documentId: string | null = null;
  private operationQueue: Promise<void> = Promise.resolve();

  constructor(
    private docsClient: IGoogleDocsClient,
    private namedRangeManager: INamedRangeManager,
    private queueManager: IQueueManager,
    private rateLimiter: ITokenBucketRateLimiter,
    private networkMonitor: INetworkMonitor,
    private stateMachine: ISyncStateMachine
  ) {}

  async startSync(documentId: string): Promise<Result<void, SyncStartError>> {
    const initResult = await this.namedRangeManager.initializeCursor(documentId);
    if (!initResult.ok) {
      return err({
        type: 'NamedRangeCreationFailed',
        message: initResult.error.message,
      });
    }

    this.documentId = documentId;
    this.stateMachine.transition('Syncing');
    return ok(undefined);
  }

  async processTranscription(message: TranscriptionMessage): Promise<Result<void, ProcessError>> {
    return this.runExclusive(async () => {
      if (!this.documentId) {
        return err({ type: 'NotStarted', message: 'Sync has not been started' });
      }

      // Always persist first to avoid data loss on transient failures
      const enqueueResult = await this.queueManager.enqueue(message);
      if (!enqueueResult.ok) {
        return err({ type: 'QueueError', message: enqueueResult.error.message });
      }

      if (!this.networkMonitor.isOnline()) {
        return ok(undefined);
      }

      const drainResult = await this.drainQueue();
      if (!drainResult.ok) {
        return err({
          type: drainResult.error.type === 'QueueReadFailed' ? 'QueueError' : 'ApiError',
          message: drainResult.error.message,
        });
      }

      return ok(undefined);
    });
  }

  async resyncOfflineQueue(): Promise<Result<void, ResyncError>> {
    return this.runExclusive(async () => {
      if (!this.documentId) {
        return err({ type: 'ResendFailed', message: 'Sync has not been started' });
      }

      const drainResult = await this.drainQueue();
      if (!drainResult.ok) {
        return err({
          type: drainResult.error.type === 'QueueReadFailed' ? 'QueueReadFailed' : 'ResendFailed',
          message: drainResult.error.message,
        });
      }

      return ok(undefined);
    });
  }

  async stopSync(): Promise<void> {
    this.documentId = null;
    this.stateMachine.reset();
  }

  private runExclusive<T>(task: () => Promise<T>): Promise<T> {
    const run = this.operationQueue.then(() => task());
    this.operationQueue = run.then(
      () => undefined,
      () => undefined
    );
    return run;
  }

  private async resolveCursor(documentId: string): Promise<Result<number, ApiError>> {
    const cursorResult = await this.namedRangeManager.getCursorPosition(documentId);
    if (cursorResult.ok) {
      return cursorResult;
    }

    if (cursorResult.error.code === 404) {
      const recoverResult = await this.namedRangeManager.recoverCursor(documentId);
      if (!recoverResult.ok) {
        return err({ code: recoverResult.error.code, message: recoverResult.error.message });
      }
      return this.namedRangeManager.getCursorPosition(documentId);
    }

    return err({ code: cursorResult.error.code, message: cursorResult.error.message });
  }

  private async drainQueue(): Promise<Result<void, { type: 'QueueReadFailed' | 'ApiFailed'; message: string }>> {
    if (!this.documentId) {
      return err({ type: 'ApiFailed', message: 'Sync has not been started' });
    }

    const queueResult = await this.queueManager.peekAll();
    if (!queueResult.ok) {
      return err({ type: 'QueueReadFailed', message: queueResult.error.message });
    }

    if (queueResult.value.length === 0) {
      return ok(undefined);
    }

    const cursorResult = await this.resolveCursor(this.documentId);
    if (!cursorResult.ok) {
      return err({ type: 'ApiFailed', message: cursorResult.error.message });
    }

    let cursor = cursorResult.value;

    for (const message of queueResult.value) {
      await this.rateLimiter.acquire();
      const insertResult = await this.docsClient.insertText(
        this.documentId,
        message.text,
        cursor
      );
      if (!insertResult.ok) {
        return err({ type: 'ApiFailed', message: insertResult.error.message });
      }

      cursor += message.text.length;
      const updateResult = await this.namedRangeManager.updateCursorPosition(this.documentId, cursor);
      if (!updateResult.ok) {
        return err({ type: 'ApiFailed', message: updateResult.error.message });
      }

      const removeResult = await this.queueManager.removeFirst(1);
      if (!removeResult.ok) {
        return err({ type: 'QueueReadFailed', message: removeResult.error.message });
      }
    }

    return ok(undefined);
  }
}
