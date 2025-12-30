/**
 * 楽観ロック制御実装
 *
 * 責務: writeControl.requiredRevisionIdによる楽観ロック
 * ファイルパス: chrome-extension/src/api/OptimisticLockHandler.ts
 *
 * Implementation: Phase 2
 *
 * 複数タブ/共同編集者との競合を防ぐ楽観ロック機能
 * 最大リトライ: 3回
 */

import { ok, err, type Result } from '../types/Result';
import type { ConflictError } from '../types/ApiTypes';
import type { IOptimisticLockHandler } from './IOptimisticLockHandler';
import type { IAuthManager } from '../auth/IAuthManager';

const DOCS_API_BASE = 'https://docs.googleapis.com/v1/documents';

export class OptimisticLockHandler implements IOptimisticLockHandler {
  private readonly maxRetries = 3;

  constructor(private authManager: IAuthManager) {}

  /**
   * 楽観ロック付きでbatchUpdateを実行
   *
   * @param documentId ドキュメントID
   * @param requests batchUpdateリクエスト配列
   * @param revisionId 期待するリビジョンID
   * @returns 成功時は新しいリビジョンID、競合時はConflictError
   */
  async batchUpdateWithLock(
    documentId: string,
    requests: unknown[],
    revisionId: string
  ): Promise<Result<string, ConflictError>> {
    const tokenResult = await this.authManager.getAccessToken();
    if (!tokenResult.ok) {
      return err({
        type: 'Conflict',
        message: tokenResult.error.message,
        currentRevisionId: '',
        attemptedRevisionId: revisionId,
      });
    }

    for (let attempt = 0; attempt < this.maxRetries; attempt++) {
      const result = await this.attemptBatchUpdate(
        documentId,
        requests,
        revisionId,
        tokenResult.value
      );

      if (result.ok) {
        return result;
      }

      // 競合エラーの場合、最新のリビジョンIDを取得してリトライ
      if (result.error.type === 'Conflict' && attempt < this.maxRetries - 1) {
        const latestRevision = await this.getLatestRevisionId(documentId, tokenResult.value);
        if (latestRevision) {
          revisionId = latestRevision;
          continue;
        }
      }

      return result;
    }

    return err({
      type: 'Conflict',
      message: 'Max retries exceeded for optimistic lock',
      currentRevisionId: '',
      attemptedRevisionId: revisionId,
    });
  }

  /**
   * batchUpdate実行を試行
   */
  private async attemptBatchUpdate(
    documentId: string,
    requests: unknown[],
    revisionId: string,
    accessToken: string
  ): Promise<Result<string, ConflictError>> {
    try {
      const response = await fetch(
        `${DOCS_API_BASE}/${documentId}:batchUpdate`,
        {
          method: 'POST',
          headers: {
            Authorization: `Bearer ${accessToken}`,
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            requests,
            writeControl: {
              requiredRevisionId: revisionId,
            },
          }),
        }
      );

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));

        // 400エラーでリビジョンミスマッチの場合
        if (response.status === 400 && errorData.error?.status === 'FAILED_PRECONDITION') {
          return err({
            type: 'Conflict',
            message: errorData.error?.message || 'Revision mismatch',
            currentRevisionId: '', // 実際の値はgetLatestRevisionIdで取得
            attemptedRevisionId: revisionId,
          });
        }

        return err({
          type: 'Conflict',
          message: errorData.error?.message || `API error (HTTP ${response.status})`,
          currentRevisionId: '',
          attemptedRevisionId: revisionId,
        });
      }

      const result = await response.json();
      // 新しいリビジョンIDを返す（batchUpdate成功時はwriteControlにrevisionIdが含まれる）
      return ok(result.writeControl?.requiredRevisionId || result.revisionId || '');
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      return err({
        type: 'Conflict',
        message,
        currentRevisionId: '',
        attemptedRevisionId: revisionId,
      });
    }
  }

  /**
   * ドキュメントの最新リビジョンIDを取得
   */
  private async getLatestRevisionId(
    documentId: string,
    accessToken: string
  ): Promise<string | null> {
    try {
      const response = await fetch(
        `${DOCS_API_BASE}/${documentId}`,
        {
          method: 'GET',
          headers: {
            Authorization: `Bearer ${accessToken}`,
          },
        }
      );

      if (!response.ok) {
        return null;
      }

      const doc = await response.json();
      return doc.revisionId || null;
    } catch {
      return null;
    }
  }
}
