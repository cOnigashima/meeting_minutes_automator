import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { ResyncOrchestrator } from '@/sync/ResyncOrchestrator';
import type { IQueueManager } from '@/sync/IQueueManager';
import type { IGoogleDocsClient } from '@/api/IGoogleDocsClient';
import type { INamedRangeManager } from '@/api/INamedRangeManager';
import type { ITokenBucketRateLimiter } from '@/sync/ITokenBucketRateLimiter';
import type { TranscriptionMessage } from '@/types/SyncTypes';
import { ok, err } from '@/types/Result';

describe('ResyncOrchestrator', () => {
  let queueManager: IQueueManager;
  let docsClient: IGoogleDocsClient;
  let namedRangeManager: INamedRangeManager;
  let rateLimiter: ITokenBucketRateLimiter;
  let orchestrator: ResyncOrchestrator;

  const message = (text: string): TranscriptionMessage => ({
    text,
    timestamp: Date.now(),
    isPartial: false,
  });

  beforeEach(() => {
    queueManager = {
      enqueue: vi.fn(),
      dequeueAll: vi.fn(),
      peekAll: vi.fn(),
      removeFirst: vi.fn(),
      clear: vi.fn(),
      size: vi.fn(),
    } as unknown as IQueueManager;
    docsClient = {
      insertText: vi.fn(),
      createNamedRange: vi.fn(),
      deleteNamedRange: vi.fn(),
      getNamedRangePosition: vi.fn(),
    } as unknown as IGoogleDocsClient;
    namedRangeManager = {
      initializeCursor: vi.fn(),
      updateCursorPosition: vi.fn(),
      recoverCursor: vi.fn(),
      getCursorPosition: vi.fn(),
    } as unknown as INamedRangeManager;
    rateLimiter = {
      acquire: vi.fn().mockResolvedValue(undefined),
      getAvailableTokens: vi.fn(),
    } as unknown as ITokenBucketRateLimiter;

    orchestrator = new ResyncOrchestrator(
      'doc-1',
      queueManager,
      docsClient,
      namedRangeManager,
      rateLimiter
    );
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should return QueueReadFailed when queue read fails', async () => {
    vi.mocked(queueManager.peekAll).mockResolvedValue(
      err({ type: 'StorageWriteFailed', message: 'read failed' })
    );

    const result = await orchestrator.resync();

    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.type).toBe('QueueReadFailed');
    }
  });

  it('should return ok when queue is empty', async () => {
    vi.mocked(queueManager.peekAll).mockResolvedValue(ok([]));

    const result = await orchestrator.resync();

    expect(result.ok).toBe(true);
    expect(docsClient.insertText).not.toHaveBeenCalled();
  });

  it('should resend messages in FIFO order', async () => {
    vi.mocked(queueManager.peekAll).mockResolvedValue(
      ok([message('first'), message('second')])
    );
    vi.mocked(queueManager.removeFirst).mockResolvedValue(ok(undefined));
    vi.mocked(namedRangeManager.getCursorPosition).mockResolvedValue(ok(1));
    vi.mocked(namedRangeManager.updateCursorPosition).mockResolvedValue(ok(undefined));
    vi.mocked(docsClient.insertText).mockResolvedValue(ok(undefined));

    const result = await orchestrator.resync();

    expect(result.ok).toBe(true);
    expect(docsClient.insertText).toHaveBeenNthCalledWith(1, 'doc-1', 'first', 1);
    expect(docsClient.insertText).toHaveBeenNthCalledWith(2, 'doc-1', 'second', 6);
    expect(rateLimiter.acquire).toHaveBeenCalledTimes(2);
    expect(queueManager.removeFirst).toHaveBeenCalledTimes(2);
  });

  it('should return ResendFailed when insert fails', async () => {
    vi.mocked(queueManager.peekAll).mockResolvedValue(ok([message('first')]));
    vi.mocked(namedRangeManager.getCursorPosition).mockResolvedValue(ok(1));
    vi.mocked(docsClient.insertText).mockResolvedValue(
      err({ code: 500, message: 'api failed' })
    );

    const result = await orchestrator.resync();

    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.type).toBe('ResendFailed');
    }
  });
});
