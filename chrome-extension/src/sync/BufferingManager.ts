/**
 * バッファリング管理実装
 *
 * Implementation: Phase 3
 */

import type { TranscriptionMessage } from '@/types/SyncTypes';
import type { IBufferingManager } from './IBufferingManager';

export class BufferingManager implements IBufferingManager {
  private messages: TranscriptionMessage[] = [];

  constructor(
    private flushHandler?: (messages: TranscriptionMessage[]) => Promise<void>
  ) {}

  buffer(message: TranscriptionMessage): void {
    this.messages.push(message);
  }

  async flush(): Promise<void> {
    if (this.messages.length === 0) return;

    const batch = [...this.messages];
    if (this.flushHandler) {
      await this.flushHandler(batch);
    }

    this.messages = [];
  }

  clear(): void {
    this.messages = [];
  }
}
