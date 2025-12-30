import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueueManager } from '@/sync/QueueManager';
import type { TranscriptionMessage } from '@/types/SyncTypes';

const mockStorage: Record<string, unknown> = {};

const mockChromeStorage = {
  local: {
    get: vi.fn(async (key: string) => ({ [key]: mockStorage[key] })),
    set: vi.fn(async (items: Record<string, unknown>) => {
      Object.assign(mockStorage, items);
    }),
    remove: vi.fn(async (key: string) => {
      delete mockStorage[key];
    }),
  },
};

vi.stubGlobal('chrome', { storage: mockChromeStorage });

describe('QueueManager', () => {
  let queueManager: QueueManager;

  const sampleMessage = (text: string): TranscriptionMessage => ({
    text,
    timestamp: Date.now(),
    isPartial: false,
  });

  beforeEach(() => {
    queueManager = new QueueManager();
    Object.keys(mockStorage).forEach(key => delete mockStorage[key]);
    vi.clearAllMocks();
  });

  describe('enqueue', () => {
    it('should save message to chrome.storage.local', async () => {
      const result = await queueManager.enqueue(sampleMessage('hello'));

      expect(result.ok).toBe(true);
      expect(mockChromeStorage.local.set).toHaveBeenCalledWith(
        expect.objectContaining({
          offline_queue: [expect.objectContaining({ text: 'hello' })],
        })
      );
    });

    it('should maintain FIFO order', async () => {
      await queueManager.enqueue(sampleMessage('first'));
      await queueManager.enqueue(sampleMessage('second'));

      const result = await queueManager.dequeueAll();

      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value.map(item => item.text)).toEqual(['first', 'second']);
      }
    });

    it('should return StorageFullError when quota exceeded', async () => {
      mockChromeStorage.local.set.mockRejectedValueOnce(
        new Error('QUOTA_BYTES quota exceeded')
      );

      const result = await queueManager.enqueue(sampleMessage('hello'));

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('QuotaExceeded');
      }
    });
  });

  describe('dequeueAll', () => {
    it('should return all messages in FIFO order', async () => {
      await queueManager.enqueue(sampleMessage('first'));
      await queueManager.enqueue(sampleMessage('second'));

      const result = await queueManager.dequeueAll();

      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value.map(item => item.text)).toEqual(['first', 'second']);
      }
    });

    it('should return empty array when queue is empty', async () => {
      const result = await queueManager.dequeueAll();

      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value).toEqual([]);
      }
    });
  });

  describe('peekAll', () => {
    it('should return messages without clearing the queue', async () => {
      await queueManager.enqueue(sampleMessage('first'));
      await queueManager.enqueue(sampleMessage('second'));

      const peekResult = await queueManager.peekAll();

      expect(peekResult.ok).toBe(true);
      if (peekResult.ok) {
        expect(peekResult.value.map(item => item.text)).toEqual(['first', 'second']);
      }

      const sizeResult = await queueManager.size();
      expect(sizeResult.ok).toBe(true);
      if (sizeResult.ok) {
        expect(sizeResult.value).toBe(2);
      }
    });
  });

  describe('removeFirst', () => {
    it('should remove the first N messages', async () => {
      await queueManager.enqueue(sampleMessage('first'));
      await queueManager.enqueue(sampleMessage('second'));
      await queueManager.enqueue(sampleMessage('third'));

      const removeResult = await queueManager.removeFirst(2);
      expect(removeResult.ok).toBe(true);

      const peekResult = await queueManager.peekAll();
      expect(peekResult.ok).toBe(true);
      if (peekResult.ok) {
        expect(peekResult.value.map(item => item.text)).toEqual(['third']);
      }
    });
  });

  describe('clear', () => {
    it('should delete queue from chrome.storage.local', async () => {
      await queueManager.enqueue(sampleMessage('hello'));

      const result = await queueManager.clear();

      expect(result.ok).toBe(true);
      expect(mockChromeStorage.local.remove).toHaveBeenCalledWith('offline_queue');
    });
  });

  describe('size', () => {
    it('should return current queue size', async () => {
      await queueManager.enqueue(sampleMessage('hello'));
      await queueManager.enqueue(sampleMessage('world'));

      const result = await queueManager.size();

      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value).toBe(2);
      }
    });
  });
});
