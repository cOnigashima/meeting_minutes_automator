import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { NamedRangeManager } from '@/api/NamedRangeManager';
import type { IGoogleDocsClient } from '@/api/IGoogleDocsClient';
import type { INamedRangeRecoveryStrategy } from '@/api/INamedRangeRecoveryStrategy';
import { ok, err } from '@/types/Result';

describe('NamedRangeManager', () => {
  let docsClient: IGoogleDocsClient;
  let recoveryStrategy: INamedRangeRecoveryStrategy;
  let manager: NamedRangeManager;

  beforeEach(() => {
    docsClient = {
      getNamedRangePosition: vi.fn(),
      createNamedRange: vi.fn(),
      deleteNamedRange: vi.fn(),
      insertText: vi.fn(),
    } as unknown as IGoogleDocsClient;
    recoveryStrategy = {
      findRecoveryPosition: vi.fn(),
    } as unknown as INamedRangeRecoveryStrategy;
    manager = new NamedRangeManager(docsClient, recoveryStrategy);
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('initializeCursor', () => {
    it('should do nothing when Named Range already exists', async () => {
      vi.mocked(docsClient.getNamedRangePosition).mockResolvedValue(
        ok({ startIndex: 10, endIndex: 11 })
      );

      const result = await manager.initializeCursor('doc-1');

      expect(result.ok).toBe(true);
      expect(docsClient.createNamedRange).not.toHaveBeenCalled();
    });

    it('should create Named Range via recovery strategy when missing', async () => {
      vi.mocked(docsClient.getNamedRangePosition).mockResolvedValue(
        err({ type: 'NotFound', message: 'missing' })
      );
      vi.mocked(recoveryStrategy.findRecoveryPosition).mockResolvedValue(ok(25));
      vi.mocked(docsClient.createNamedRange).mockResolvedValue(ok(undefined));

      const result = await manager.initializeCursor('doc-1');

      expect(result.ok).toBe(true);
      expect(recoveryStrategy.findRecoveryPosition).toHaveBeenCalledWith('doc-1');
      expect(docsClient.createNamedRange).toHaveBeenCalledWith(
        'doc-1',
        'transcript_cursor',
        25,
        26
      );
    });
  });

  describe('updateCursorPosition', () => {
    it('should delete and recreate Named Range at new position', async () => {
      vi.mocked(docsClient.deleteNamedRange).mockResolvedValue(ok(undefined));
      vi.mocked(docsClient.createNamedRange).mockResolvedValue(ok(undefined));

      const result = await manager.updateCursorPosition('doc-1', 42);

      expect(result.ok).toBe(true);
      expect(docsClient.deleteNamedRange).toHaveBeenCalledWith('doc-1', 'transcript_cursor');
      expect(docsClient.createNamedRange).toHaveBeenCalledWith(
        'doc-1',
        'transcript_cursor',
        42,
        43
      );
    });

    it('should return error when delete fails', async () => {
      vi.mocked(docsClient.deleteNamedRange).mockResolvedValue(
        err({ code: 500, message: 'delete failed' })
      );

      const result = await manager.updateCursorPosition('doc-1', 42);

      expect(result.ok).toBe(false);
      expect(docsClient.createNamedRange).not.toHaveBeenCalled();
    });
  });

  describe('recoverCursor', () => {
    it('should create Named Range at recovery position', async () => {
      vi.mocked(recoveryStrategy.findRecoveryPosition).mockResolvedValue(ok(7));
      vi.mocked(docsClient.createNamedRange).mockResolvedValue(ok(undefined));

      const result = await manager.recoverCursor('doc-1');

      expect(result.ok).toBe(true);
      expect(docsClient.createNamedRange).toHaveBeenCalledWith(
        'doc-1',
        'transcript_cursor',
        7,
        8
      );
    });

    it('should return error when recovery strategy fails', async () => {
      vi.mocked(recoveryStrategy.findRecoveryPosition).mockResolvedValue(
        err({ code: 404, message: 'not found' })
      );

      const result = await manager.recoverCursor('doc-1');

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.code).toBe(500);
      }
      expect(docsClient.createNamedRange).not.toHaveBeenCalled();
    });
  });

  describe('getCursorPosition', () => {
    it('should return current cursor position', async () => {
      vi.mocked(docsClient.getNamedRangePosition).mockResolvedValue(
        ok({ startIndex: 12, endIndex: 13 })
      );

      const result = await manager.getCursorPosition('doc-1');

      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value).toBe(12);
      }
    });

    it('should return ApiError when cursor is missing', async () => {
      vi.mocked(docsClient.getNamedRangePosition).mockResolvedValue(
        err({ type: 'NotFound', message: 'missing' })
      );

      const result = await manager.getCursorPosition('doc-1');

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.code).toBe(404);
      }
    });
  });
});
