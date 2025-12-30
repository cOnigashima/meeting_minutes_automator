/**
 * 同期状態遷移管理インターフェース
 *
 * 責務: 同期状態（NotStarted/Syncing/Paused/Error）の遷移ロジック
 * テスト戦略: 完全にモック可能（⭐⭐⭐⭐⭐）
 */

import type { Result } from '@/types/Result';
import type { SyncState, InvalidTransitionError } from '@/types/SyncTypes';

export interface ISyncStateMachine {
  getCurrentState(): SyncState;
  transition(toState: SyncState): Result<void, InvalidTransitionError>;
  reset(): void;
}
