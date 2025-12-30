import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { NamedRangeRecoveryStrategy } from '@/api/NamedRangeRecoveryStrategy';
import type { IAuthManager } from '@/auth/IAuthManager';
import { ok, err } from '@/types/Result';

const mockFetch = vi.fn();
vi.stubGlobal('fetch', mockFetch);

describe('NamedRangeRecoveryStrategy', () => {
  let strategy: NamedRangeRecoveryStrategy;
  let authManager: IAuthManager;

  beforeEach(() => {
    authManager = { getAccessToken: vi.fn() } as unknown as IAuthManager;
    strategy = new NamedRangeRecoveryStrategy(authManager);
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should return 401 when access token is missing', async () => {
    vi.mocked(authManager.getAccessToken).mockResolvedValue(
      err({ type: 'RefreshRequired', message: 'no token' })
    );

    const result = await strategy.findRecoveryPosition('doc-1');

    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.code).toBe(401);
    }
  });

  it('should return heading position when heading is found', async () => {
    vi.mocked(authManager.getAccessToken).mockResolvedValue(ok('token-123'));
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        body: {
          content: [
            {
              paragraph: {
                elements: [{ textRun: { content: 'Intro\n## 文字起こし\n' } }],
              },
              endIndex: 20,
            },
          ],
        },
      }),
    });

    const result = await strategy.findRecoveryPosition('doc-1');

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value).toBe(20);
    }
  });

  it('should fallback to document end when heading is missing', async () => {
    vi.mocked(authManager.getAccessToken).mockResolvedValue(ok('token-123'));
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        body: {
          content: [{ endIndex: 50 }],
        },
      }),
    });

    const result = await strategy.findRecoveryPosition('doc-1');

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value).toBe(49);
    }
  });

  it('should fallback to document start when document is empty', async () => {
    vi.mocked(authManager.getAccessToken).mockResolvedValue(ok('token-123'));
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        body: {
          content: [],
        },
      }),
    });

    const result = await strategy.findRecoveryPosition('doc-1');

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value).toBe(1);
    }
  });
});
