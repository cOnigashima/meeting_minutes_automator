/**
 * 楽観ロック制御インターフェース
 *
 * 責務: writeControl.requiredRevisionIdによる楽観ロック
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */

import type { Result } from '@/types/Result';
import type { ConflictError } from '@/types/ApiTypes';

export interface IOptimisticLockHandler {
  batchUpdateWithLock(
    documentId: string,
    requests: any[],
    revisionId: string
  ): Promise<Result<string, ConflictError>>;
}
