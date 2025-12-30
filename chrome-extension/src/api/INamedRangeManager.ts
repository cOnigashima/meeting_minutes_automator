/**
 * Named Range統合インターフェース
 *
 * 責務: Named Range作成、更新、自動復旧
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */

import type { Result } from '@/types/Result';
import type { ApiError } from '@/types/ApiTypes';

export interface INamedRangeManager {
  initializeCursor(documentId: string): Promise<Result<void, ApiError>>;
  updateCursorPosition(documentId: string, newIndex: number): Promise<Result<void, ApiError>>;
  recoverCursor(documentId: string): Promise<Result<void, ApiError>>;
  getCursorPosition(documentId: string): Promise<Result<number, ApiError>>;
}
