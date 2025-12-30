/**
 * 同期フロー統合インターフェース
 *
 * 責務: 文字起こしメッセージ受信、オンライン/オフライン状態管理、自動同期制御
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */

import type { Result } from '@/types/Result';
import type { TranscriptionMessage, SyncStartError, ProcessError, ResyncError } from '@/types/SyncTypes';

export interface ISyncManager {
  startSync(documentId: string): Promise<Result<void, SyncStartError>>;
  processTranscription(message: TranscriptionMessage): Promise<Result<void, ProcessError>>;
  resyncOfflineQueue(): Promise<Result<void, ResyncError>>;
  stopSync(): Promise<void>;
}
