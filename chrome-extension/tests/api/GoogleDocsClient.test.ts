import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { GoogleDocsClient } from '@/api/GoogleDocsClient';
import { ExponentialBackoffHandler } from '@/api/ExponentialBackoffHandler';
import type { IAuthManager } from '@/auth/IAuthManager';
import { ok, err } from '@/types/Result';

const mockFetch = vi.fn();
vi.stubGlobal('fetch', mockFetch);

describe('GoogleDocsClient', () => {
  let client: GoogleDocsClient;
  let mockAuthManager: IAuthManager;
  let backoffHandler: ExponentialBackoffHandler;

  beforeEach(() => {
    mockAuthManager = { getAccessToken: vi.fn() } as unknown as IAuthManager;
    backoffHandler = new ExponentialBackoffHandler();
    vi.spyOn(backoffHandler, 'executeWithBackoff').mockImplementation(async (fn) => ok(await fn()));
    client = new GoogleDocsClient(mockAuthManager, backoffHandler);
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('insertText', () => {
    it('should POST to documents.batchUpdate endpoint with insertText request', async () => {
      vi.mocked(mockAuthManager.getAccessToken).mockResolvedValue(ok('token-123'));
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({}),
      });

      const result = await client.insertText('doc-1', 'Hello', 5);

      expect(result.ok).toBe(true);
      expect(mockFetch).toHaveBeenCalledTimes(1);
      const [url, options] = mockFetch.mock.calls[0];
      expect(url).toBe('https://docs.googleapis.com/v1/documents/doc-1:batchUpdate');
      expect(options.method).toBe('POST');
      expect(options.headers.Authorization).toBe('Bearer token-123');
      const body = JSON.parse(options.body as string);
      expect(body.requests[0].insertText.location.index).toBe(5);
      expect(body.requests[0].insertText.text).toBe('Hello');
    });

    it('should return ApiError when access token is missing', async () => {
      vi.mocked(mockAuthManager.getAccessToken).mockResolvedValue(
        err({ type: 'RefreshRequired', message: 'no token' })
      );

      const result = await client.insertText('doc-1', 'Hello', 1);

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.code).toBe(401);
        expect(result.error.status).toBe('UNAUTHENTICATED');
      }
      expect(mockFetch).not.toHaveBeenCalled();
    });

    it('should map API error response to ApiError', async () => {
      vi.mocked(mockAuthManager.getAccessToken).mockResolvedValue(ok('token-123'));
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 403,
        json: async () => ({ error: { message: 'Forbidden', status: 'PERMISSION_DENIED' } }),
      });

      const result = await client.insertText('doc-1', 'Hello', 1);

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.code).toBe(403);
        expect(result.error.status).toBe('PERMISSION_DENIED');
        expect(result.error.message).toBe('Forbidden');
      }
    });
  });

  describe('createNamedRange', () => {
    it('should create Named Range with specified name and range', async () => {
      vi.mocked(mockAuthManager.getAccessToken).mockResolvedValue(ok('token-123'));
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({}),
      });

      const result = await client.createNamedRange('doc-1', 'range-1', 10, 20);

      expect(result.ok).toBe(true);
      const [, options] = mockFetch.mock.calls[0];
      const body = JSON.parse(options.body as string);
      expect(body.requests[0].createNamedRange.name).toBe('range-1');
      expect(body.requests[0].createNamedRange.range.startIndex).toBe(10);
      expect(body.requests[0].createNamedRange.range.endIndex).toBe(20);
    });
  });

  describe('getNamedRangePosition', () => {
    it('should GET document and parse Named Range position', async () => {
      vi.mocked(mockAuthManager.getAccessToken).mockResolvedValue(ok('token-123'));
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          namedRanges: {
            cursor: {
              namedRanges: [
                {
                  ranges: [{ startIndex: 5, endIndex: 8 }],
                },
              ],
            },
          },
        }),
      });

      const result = await client.getNamedRangePosition('doc-1', 'cursor');

      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value.startIndex).toBe(5);
        expect(result.value.endIndex).toBe(8);
      }
    });

    it('should return NotFoundError when Named Range does not exist', async () => {
      vi.mocked(mockAuthManager.getAccessToken).mockResolvedValue(ok('token-123'));
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ namedRanges: {} }),
      });

      const result = await client.getNamedRangePosition('doc-1', 'missing');

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error.type).toBe('NotFound');
      }
    });
  });

  describe('deleteNamedRange', () => {
    it('should return ok when Named Range is missing', async () => {
      vi.mocked(mockAuthManager.getAccessToken).mockResolvedValue(ok('token-123'));
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ namedRanges: {} }),
      });

      const result = await client.deleteNamedRange('doc-1', 'missing');

      expect(result.ok).toBe(true);
      expect(mockFetch).toHaveBeenCalledTimes(1);
    });

    it('should delete all Named Range IDs', async () => {
      vi.mocked(mockAuthManager.getAccessToken).mockResolvedValue(ok('token-123'));
      mockFetch
        .mockResolvedValueOnce({
          ok: true,
          json: async () => ({
            namedRanges: {
              cursor: {
                namedRanges: [{ namedRangeId: 'nr-1' }, { namedRangeId: 'nr-2' }],
              },
            },
          }),
        })
        .mockResolvedValueOnce({
          ok: true,
          json: async () => ({}),
        });

      const result = await client.deleteNamedRange('doc-1', 'cursor');

      expect(result.ok).toBe(true);
      expect(mockFetch).toHaveBeenCalledTimes(2);
      const [, options] = mockFetch.mock.calls[1];
      const body = JSON.parse(options.body as string);
      expect(body.requests).toHaveLength(2);
      expect(body.requests[0].deleteNamedRange.namedRangeId).toBe('nr-1');
      expect(body.requests[1].deleteNamedRange.namedRangeId).toBe('nr-2');
    });
  });
});
