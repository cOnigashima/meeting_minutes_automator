import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { SyncManager } from '@/sync/SyncManager';
import type { IGoogleDocsClient } from '@/api/IGoogleDocsClient';
import type { INamedRangeManager } from '@/api/INamedRangeManager';
import type { IQueueManager } from '@/sync/IQueueManager';
import type { ITokenBucketRateLimiter } from '@/sync/ITokenBucketRateLimiter';
import type { INetworkMonitor } from '@/sync/INetworkMonitor';
import type { ISyncStateMachine } from '@/sync/ISyncStateMachine';
import type { TranscriptionMessage } from '@/types/SyncTypes';
import { ok, err } from '@/types/Result';

describe('SyncManager', () => {
  let docsClient: IGoogleDocsClient;
  let namedRangeManager: INamedRangeManager;
  let queueManager: IQueueManager;
  let rateLimiter: ITokenBucketRateLimiter;
  let networkMonitor: INetworkMonitor;
  let stateMachine: ISyncStateMachine;
  let manager: SyncManager;

  const message = (text: string): TranscriptionMessage => ({
    text,
    timestamp: Date.now(),
    isPartial: false,
  });

  beforeEach(() => {
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
    queueManager = {
      enqueue: vi.fn(),
      dequeueAll: vi.fn(),
      peekAll: vi.fn(),
      removeFirst: vi.fn(),
      clear: vi.fn(),
      size: vi.fn(),
    } as unknown as IQueueManager;
    rateLimiter = {
      acquire: vi.fn().mockResolvedValue(undefined),
      getAvailableTokens: vi.fn(),
    } as unknown as ITokenBucketRateLimiter;
    networkMonitor = {
      isOnline: vi.fn(),
      onStateChange: vi.fn(),
      removeStateChangeListener: vi.fn(),
    } as unknown as INetworkMonitor;
    stateMachine = {
      getCurrentState: vi.fn(),
      transition: vi.fn().mockReturnValue(ok(undefined)),
      reset: vi.fn(),
    } as unknown as ISyncStateMachine;

    manager = new SyncManager(
      docsClient,
      namedRangeManager,
      queueManager,
      rateLimiter,
      networkMonitor,
      stateMachine
    );
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('startSync', () => {
    it('should create Named Range via NamedRangeManager', async () => {
      vi.mocked(namedRangeManager.initializeCursor).mockResolvedValue(ok(undefined));

      const result = await manager.startSync('doc-1');

      expect(result.ok).toBe(true);
      expect(namedRangeManager.initializeCursor).toHaveBeenCalledWith('doc-1');
      expect(stateMachine.transition).toHaveBeenCalledWith('Syncing');
    });

    it('should return SyncStartError when Named Range creation fails', async () => {
      vi.mocked(namedRangeManager.initializeCursor).mockResolvedValue(
        err({ code: 500, message: 'failed' })
      );

      const result = await manager.startSync('doc-1');

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('NamedRangeCreationFailed');
      }
    });
  });

  describe('processTranscription', () => {
    it('should return ProcessError when sync not started', async () => {
      const result = await manager.processTranscription(message('hello'));

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('NotStarted');
      }
    });

    it('should send message to GoogleDocsClient when online', async () => {
      vi.mocked(namedRangeManager.initializeCursor).mockResolvedValue(ok(undefined));
      await manager.startSync('doc-1');

      vi.mocked(networkMonitor.isOnline).mockReturnValue(true);
      vi.mocked(namedRangeManager.getCursorPosition).mockResolvedValue(ok(10));
      vi.mocked(queueManager.enqueue).mockResolvedValue(ok(undefined));
      vi.mocked(queueManager.peekAll).mockResolvedValue(ok([message('hello')]));
      vi.mocked(queueManager.removeFirst).mockResolvedValue(ok(undefined));
      vi.mocked(docsClient.insertText).mockResolvedValue(ok(undefined));
      vi.mocked(namedRangeManager.updateCursorPosition).mockResolvedValue(ok(undefined));

      const result = await manager.processTranscription(message('hello'));

      expect(result.ok).toBe(true);
      expect(rateLimiter.acquire).toHaveBeenCalled();
      expect(queueManager.enqueue).toHaveBeenCalled();
      expect(queueManager.removeFirst).toHaveBeenCalledWith(1);
      expect(docsClient.insertText).toHaveBeenCalledWith('doc-1', 'hello', 10);
      expect(namedRangeManager.updateCursorPosition).toHaveBeenCalledWith('doc-1', 15);
    });

    it('should enqueue message to QueueManager when offline', async () => {
      vi.mocked(namedRangeManager.initializeCursor).mockResolvedValue(ok(undefined));
      await manager.startSync('doc-1');

      vi.mocked(networkMonitor.isOnline).mockReturnValue(false);
      vi.mocked(queueManager.enqueue).mockResolvedValue(ok(undefined));

      const result = await manager.processTranscription(message('offline'));

      expect(result.ok).toBe(true);
      expect(queueManager.enqueue).toHaveBeenCalled();
      expect(docsClient.insertText).not.toHaveBeenCalled();
    });

    it('should enqueue message and return ApiError when API fails', async () => {
      vi.mocked(namedRangeManager.initializeCursor).mockResolvedValue(ok(undefined));
      await manager.startSync('doc-1');

      vi.mocked(networkMonitor.isOnline).mockReturnValue(true);
      vi.mocked(queueManager.enqueue).mockResolvedValue(ok(undefined));
      vi.mocked(queueManager.peekAll).mockResolvedValue(ok([message('hello')]));
      vi.mocked(namedRangeManager.getCursorPosition).mockResolvedValue(ok(10));
      vi.mocked(docsClient.insertText).mockResolvedValue(
        err({ code: 503, message: 'Service Unavailable' })
      );

      const result = await manager.processTranscription(message('hello'));

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('ApiError');
      }
      expect(queueManager.enqueue).toHaveBeenCalled();
      expect(queueManager.removeFirst).not.toHaveBeenCalled();
    });

    it('should recover cursor when Named Range is missing', async () => {
      vi.mocked(namedRangeManager.initializeCursor).mockResolvedValue(ok(undefined));
      await manager.startSync('doc-1');

      vi.mocked(networkMonitor.isOnline).mockReturnValue(true);
      vi.mocked(queueManager.enqueue).mockResolvedValue(ok(undefined));
      vi.mocked(queueManager.peekAll).mockResolvedValue(ok([message('hello')]));
      vi.mocked(queueManager.removeFirst).mockResolvedValue(ok(undefined));
      vi.mocked(namedRangeManager.getCursorPosition)
        .mockResolvedValueOnce(err({ code: 404, message: 'missing' }))
        .mockResolvedValueOnce(ok(7));
      vi.mocked(namedRangeManager.recoverCursor).mockResolvedValue(ok(undefined));
      vi.mocked(docsClient.insertText).mockResolvedValue(ok(undefined));
      vi.mocked(namedRangeManager.updateCursorPosition).mockResolvedValue(ok(undefined));

      const result = await manager.processTranscription(message('hello'));

      expect(result.ok).toBe(true);
      expect(namedRangeManager.recoverCursor).toHaveBeenCalledWith('doc-1');
    });
  });

  describe('resyncOfflineQueue', () => {
    it('should dequeue all messages and send them in FIFO order', async () => {
      vi.mocked(namedRangeManager.initializeCursor).mockResolvedValue(ok(undefined));
      await manager.startSync('doc-1');

      vi.mocked(queueManager.peekAll).mockResolvedValue(ok([message('first'), message('second')]));
      vi.mocked(queueManager.removeFirst).mockResolvedValue(ok(undefined));
      vi.mocked(namedRangeManager.getCursorPosition).mockResolvedValue(ok(1));
      vi.mocked(namedRangeManager.updateCursorPosition).mockResolvedValue(ok(undefined));
      vi.mocked(docsClient.insertText).mockResolvedValue(ok(undefined));

      const result = await manager.resyncOfflineQueue();

      expect(result.ok).toBe(true);
      expect(docsClient.insertText).toHaveBeenNthCalledWith(1, 'doc-1', 'first', 1);
      expect(docsClient.insertText).toHaveBeenNthCalledWith(2, 'doc-1', 'second', 6);
      expect(queueManager.removeFirst).toHaveBeenCalledTimes(2);
    });

    it('should return ResyncError when queue read fails', async () => {
      vi.mocked(namedRangeManager.initializeCursor).mockResolvedValue(ok(undefined));
      await manager.startSync('doc-1');

      vi.mocked(queueManager.peekAll).mockResolvedValue(
        err({ type: 'StorageWriteFailed', message: 'fail' })
      );

      const result = await manager.resyncOfflineQueue();

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('QueueReadFailed');
      }
    });
  });

  describe('stopSync', () => {
    it('should reset sync state to NotStarted', async () => {
      vi.mocked(namedRangeManager.initializeCursor).mockResolvedValue(ok(undefined));
      await manager.startSync('doc-1');

      await manager.stopSync();
      const result = await manager.processTranscription(message('hello'));

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('NotStarted');
      }
      expect(stateMachine.reset).toHaveBeenCalled();
    });
  });
});
