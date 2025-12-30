/**
 * Named Range自動復旧戦略インターフェース
 *
 * 責務: Named Range削除検知、復旧位置決定
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 *
 * 復旧優先度:
 *   1. 見出し「## 文字起こし」検索 → 見出し直後
 *   2. ドキュメント末尾
 *   3. ドキュメント先頭（index=1）
 */

import type { Result } from '@/types/Result';
import type { ApiError } from '@/types/ApiTypes';

export interface INamedRangeRecoveryStrategy {
  /**
   * 復旧位置を検索
   * @returns 復旧すべきインデックス位置
   */
  findRecoveryPosition(documentId: string): Promise<Result<number, ApiError>>;
}
