/**
 * Google Docs API呼び出し統合インターフェース
 *
 * 責務: documents.batchUpdate呼び出し、リトライ、楽観ロック
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */

import type { Result } from '@/types/Result';
import type { ApiError, NotFoundError } from '@/types/ApiTypes';

export interface IGoogleDocsClient {
  insertText(
    documentId: string,
    text: string,
    index: number
  ): Promise<Result<void, ApiError>>;

  createNamedRange(
    documentId: string,
    name: string,
    startIndex: number,
    endIndex: number
  ): Promise<Result<void, ApiError>>;

  deleteNamedRange(
    documentId: string,
    name: string
  ): Promise<Result<void, ApiError>>;

  getNamedRangePosition(
    documentId: string,
    name: string
  ): Promise<Result<{ startIndex: number; endIndex: number }, NotFoundError>>;
}
