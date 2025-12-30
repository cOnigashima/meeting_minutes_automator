/**
 * Google Docs API呼び出し統合実装
 *
 * 責務: Google Docs API v1のラッパー（テキスト挿入、Named Range管理）
 * ファイルパス: chrome-extension/src/api/GoogleDocsClient.ts
 *
 * Implementation: Phase 2
 *
 * エンドポイント: https://docs.googleapis.com/v1/documents
 * 認証: Bearer token (AuthManagerから取得)
 */

import { ok, err, type Result } from '../types/Result';
import type { ApiError, NotFoundError } from '../types/ApiTypes';
import type { IGoogleDocsClient } from './IGoogleDocsClient';
import type { IAuthManager } from '../auth/IAuthManager';
import { ExponentialBackoffHandler, RetryableError } from './ExponentialBackoffHandler';

const DOCS_API_BASE = 'https://docs.googleapis.com/v1/documents';

export class GoogleDocsClient implements IGoogleDocsClient {
  private readonly backoffHandler: ExponentialBackoffHandler;

  constructor(
    private authManager: IAuthManager,
    backoffHandler?: ExponentialBackoffHandler
  ) {
    this.backoffHandler = backoffHandler ?? new ExponentialBackoffHandler();
  }

  /**
   * ドキュメントにテキストを挿入
   */
  async insertText(
    documentId: string,
    text: string,
    index: number
  ): Promise<Result<void, ApiError>> {
    const tokenResult = await this.authManager.getAccessToken();
    if (!tokenResult.ok) {
      return err({
        code: 401,
        message: tokenResult.error.message,
        status: 'UNAUTHENTICATED',
      });
    }

    const request = {
      requests: [
        {
          insertText: {
            location: { index },
            text,
          },
        },
      ],
    };

    return this.batchUpdate(documentId, request, tokenResult.value);
  }

  /**
   * Named Rangeを作成
   */
  async createNamedRange(
    documentId: string,
    name: string,
    startIndex: number,
    endIndex: number
  ): Promise<Result<void, ApiError>> {
    const tokenResult = await this.authManager.getAccessToken();
    if (!tokenResult.ok) {
      return err({
        code: 401,
        message: tokenResult.error.message,
        status: 'UNAUTHENTICATED',
      });
    }

    const request = {
      requests: [
        {
          createNamedRange: {
            name,
            range: {
              startIndex,
              endIndex,
            },
          },
        },
      ],
    };

    return this.batchUpdate(documentId, request, tokenResult.value);
  }

  /**
   * Named Rangeの位置を取得
   */
  async getNamedRangePosition(
    documentId: string,
    name: string
  ): Promise<Result<{ startIndex: number; endIndex: number }, NotFoundError>> {
    const tokenResult = await this.authManager.getAccessToken();
    if (!tokenResult.ok) {
      return err({
        type: 'NotFound',
        message: tokenResult.error.message,
      });
    }

    try {
      const response = await this.fetchWithBackoff(
        `${DOCS_API_BASE}/${documentId}`,
        {
          method: 'GET',
          headers: {
            Authorization: `Bearer ${tokenResult.value}`,
          },
        }
      );

      if (!response.ok) {
        return err({
          type: 'NotFound',
          message: `Document not found or access denied (HTTP ${response.status})`,
        });
      }

      const doc = await response.json();
      const namedRanges = doc.namedRanges?.[name];

      if (!namedRanges || namedRanges.namedRanges.length === 0) {
        return err({
          type: 'NotFound',
          message: `Named range '${name}' not found`,
        });
      }

      // 最初のNamed Rangeを使用
      const range = namedRanges.namedRanges[0].ranges[0];
      return ok({
        startIndex: range.startIndex,
        endIndex: range.endIndex,
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      return err({
        type: 'NotFound',
        message,
      });
    }
  }

  /**
   * Named Rangeを削除
   */
  async deleteNamedRange(
    documentId: string,
    name: string
  ): Promise<Result<void, ApiError>> {
    // まずNamed RangeのIDを取得
    const tokenResult = await this.authManager.getAccessToken();
    if (!tokenResult.ok) {
      return err({
        code: 401,
        message: tokenResult.error.message,
        status: 'UNAUTHENTICATED',
      });
    }

    try {
      const response = await this.fetchWithBackoff(
        `${DOCS_API_BASE}/${documentId}`,
        {
          method: 'GET',
          headers: {
            Authorization: `Bearer ${tokenResult.value}`,
          },
        }
      );

      if (!response.ok) {
        return err({
          code: response.status,
          message: `Failed to get document (HTTP ${response.status})`,
        });
      }

      const doc = await response.json();
      const namedRanges = doc.namedRanges?.[name];

      if (!namedRanges || namedRanges.namedRanges.length === 0) {
        // Named Rangeが存在しない場合は成功扱い
        return ok(undefined);
      }

      // 全てのNamed Rangeを削除
      const deleteRequests = namedRanges.namedRanges.map((nr: { namedRangeId: string }) => ({
        deleteNamedRange: {
          namedRangeId: nr.namedRangeId,
        },
      }));

      const request = { requests: deleteRequests };
      return this.batchUpdate(documentId, request, tokenResult.value);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      return err({
        code: 500,
        message,
      });
    }
  }

  /**
   * batchUpdate APIを呼び出し
   */
  private async batchUpdate(
    documentId: string,
    request: { requests: unknown[] },
    accessToken: string
  ): Promise<Result<void, ApiError>> {
    try {
      const response = await this.fetchWithBackoff(
        `${DOCS_API_BASE}/${documentId}:batchUpdate`,
        {
          method: 'POST',
          headers: {
            Authorization: `Bearer ${accessToken}`,
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(request),
        }
      );

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        return err({
          code: response.status,
          message: errorData.error?.message || `API error (HTTP ${response.status})`,
          status: errorData.error?.status,
        });
      }

      return ok(undefined);
    } catch (error) {
      if (error instanceof RetryableError) {
        return err({
          code: error.statusCode,
          message: error.message,
        });
      }
      const message = error instanceof Error ? error.message : String(error);
      return err({
        code: 500,
        message,
      });
    }
  }

  /**
   * Exponential Backoff付きでfetchを実行
   */
  private async fetchWithBackoff(
    url: string,
    options: RequestInit
  ): Promise<Response> {
    const result = await this.backoffHandler.executeWithBackoff(async () => {
      const response = await fetch(url, options);

      if (ExponentialBackoffHandler.isRetryableStatus(response.status)) {
        throw new RetryableError(
          `Retryable error (HTTP ${response.status})`,
          response.status
        );
      }

      return response;
    });

    if (!result.ok) {
      throw new Error(result.error.message);
    }

    return result.value;
  }
}
