/**
 * 再同期制御インターフェース
 *
 * 責務: オフラインキュー再同期の制御、レート制限遵守
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */

import type { Result } from '@/types/Result';
import type { ResyncError } from '@/types/SyncTypes';

export interface IResyncOrchestrator {
  resync(): Promise<Result<void, ResyncError>>;
}
