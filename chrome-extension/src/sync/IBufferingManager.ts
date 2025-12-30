/**
 * バッファリング管理インターフェース
 *
 * 責務: 短時間のメッセージをバッファリング、一括送信
 * テスト戦略: 完全にモック可能（⭐⭐⭐⭐⭐）
 */

import type { TranscriptionMessage } from '@/types/SyncTypes';

export interface IBufferingManager {
  buffer(message: TranscriptionMessage): void;
  flush(): Promise<void>;
  clear(): void;
}
