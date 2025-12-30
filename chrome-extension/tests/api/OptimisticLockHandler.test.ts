import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { OptimisticLockHandler } from '@/api/OptimisticLockHandler';
import type { IAuthManager } from '@/auth/IAuthManager';
import { ok, err } from '@/types/Result';

const mockFetch = vi.fn();
vi.stubGlobal('fetch', mockFetch);

describe('OptimisticLockHandler', () => {
  let handler: OptimisticLockHandler;
  let authManager: IAuthManager;

  beforeEach(() => {
    authManager = { getAccessToken: vi.fn() } as unknown as IAuthManager;
    handler = new OptimisticLockHandler(authManager);
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should return Conflict when access token is missing', async () => {
    vi.mocked(authManager.getAccessToken).mockResolvedValue(
      err({ type: 'RefreshRequired', message: 'no token' })
    );

    const result = await handler.batchUpdateWithLock('doc-1', [], 'rev-1');

    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.type).toBe('Conflict');
      expect(result.error.attemptedRevisionId).toBe('rev-1');
    }
  });

  it('should return new revisionId on successful batchUpdate', async () => {
    vi.mocked(authManager.getAccessToken).mockResolvedValue(ok('token-123'));
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ writeControl: { requiredRevisionId: 'rev-2' } }),
    });

    const result = await handler.batchUpdateWithLock('doc-1', [{ insertText: {} }], 'rev-1');

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value).toBe('rev-2');
    }
  });

  it('should retry with latest revision on conflict', async () => {
    vi.mocked(authManager.getAccessToken).mockResolvedValue(ok('token-123'));
    mockFetch
      .mockResolvedValueOnce({
        ok: false,
        status: 400,
        json: async () => ({
          error: { status: 'FAILED_PRECONDITION', message: 'Revision mismatch' },
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ revisionId: 'rev-2' }),
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({}),
      });

    const result = await handler.batchUpdateWithLock('doc-1', [{ insertText: {} }], 'rev-1');

    expect(result.ok).toBe(true);
    expect(mockFetch).toHaveBeenCalledTimes(3);
    const secondPost = mockFetch.mock.calls[2];
    const body = JSON.parse(secondPost[1].body as string);
    expect(body.writeControl.requiredRevisionId).toBe('rev-2');
  });
});
