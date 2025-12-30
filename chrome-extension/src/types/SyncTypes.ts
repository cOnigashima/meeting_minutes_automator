/**
 * Sync Domain Types
 */

export type SyncState = 'NotStarted' | 'Syncing' | 'Paused' | 'Error';

export type TranscriptionMessage = {
  text: string;
  timestamp: number;
  isPartial: boolean;
  confidence?: number;
  language?: string;
};

export type SyncStartError = {
  type: 'DocumentIdFetchFailed' | 'NamedRangeCreationFailed';
  message: string;
};

export type ProcessError = {
  type: 'NotStarted' | 'ApiError' | 'QueueError';
  message: string;
};

export type ResyncError = {
  type: 'ResendFailed' | 'QueueReadFailed';
  message: string;
};

export type InvalidTransitionError = {
  type: 'InvalidTransition';
  from: SyncState;
  to: SyncState;
  message: string;
};

export type StorageFullError = {
  type: 'QuotaExceeded';
  message: string;
};

/**
 * Google Docs Sync Settings (DOCS-REQ-008)
 */
export interface DocsSyncSettings {
  /** Google Docs同期の有効/無効 (default: true) */
  enabled: boolean;
  /** タイムスタンプ表示 (default: true) */
  showTimestamp: boolean;
  /** 話者名表示 (default: false) */
  showSpeaker: boolean;
  /** バッファリング時間（秒）(default: 3, range: 1-5) */
  bufferingSeconds: number;
  /** 同期先ドキュメントID (optional) */
  documentId?: string;
}

export const DEFAULT_DOCS_SYNC_SETTINGS: DocsSyncSettings = {
  enabled: true,
  showTimestamp: true,
  showSpeaker: false,
  bufferingSeconds: 3,
};
