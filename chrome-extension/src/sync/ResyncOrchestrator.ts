/**
 * 再同期制御実装
 *
 * Implementation: Phase 3
 */

import type { Result } from '@/types/Result';
import type { ResyncError } from '@/types/SyncTypes';
import type { IResyncOrchestrator } from './IResyncOrchestrator';
import type { IQueueManager } from './IQueueManager';
import type { IGoogleDocsClient } from '@/api/IGoogleDocsClient';
import type { INamedRangeManager } from '@/api/INamedRangeManager';
import type { ITokenBucketRateLimiter } from './ITokenBucketRateLimiter';
import { ok, err } from '@/types/Result';

export class ResyncOrchestrator implements IResyncOrchestrator {
  constructor(
    private documentId: string,
    private queueManager: IQueueManager,
    private docsClient: IGoogleDocsClient,
    private namedRangeManager: INamedRangeManager,
    private rateLimiter: ITokenBucketRateLimiter
  ) {}

  async resync(): Promise<Result<void, ResyncError>> {
    const queueResult = await this.queueManager.peekAll();
    if (!queueResult.ok) {
      return err({ type: 'QueueReadFailed', message: queueResult.error.message });
    }

    if (queueResult.value.length === 0) {
      return ok(undefined);
    }

    let cursorResult = await this.namedRangeManager.getCursorPosition(this.documentId);
    if (!cursorResult.ok) {
      if (cursorResult.error.code === 404) {
        const recoverResult = await this.namedRangeManager.recoverCursor(this.documentId);
        if (!recoverResult.ok) {
          return err({ type: 'ResendFailed', message: recoverResult.error.message });
        }
        cursorResult = await this.namedRangeManager.getCursorPosition(this.documentId);
      }
    }

    if (!cursorResult.ok) {
      return err({ type: 'ResendFailed', message: cursorResult.error.message });
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
        return err({ type: 'ResendFailed', message: insertResult.error.message });
      }
      cursor += message.text.length;
      const updateResult = await this.namedRangeManager.updateCursorPosition(
        this.documentId,
        cursor
      );
      if (!updateResult.ok) {
        return err({ type: 'ResendFailed', message: updateResult.error.message });
      }

      const removeResult = await this.queueManager.removeFirst(1);
      if (!removeResult.ok) {
        return err({ type: 'QueueReadFailed', message: removeResult.error.message });
      }
    }

    return ok(undefined);
  }
}
