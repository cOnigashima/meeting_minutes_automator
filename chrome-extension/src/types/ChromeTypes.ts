/**
 * Chrome API Error Types
 */

export type ChromeError = {
  type: 'ChromeRuntimeError';
  message: string;
  lastError?: chrome.runtime.LastError;
};

export type AlarmError = {
  type: 'AlarmCreateFailed' | 'AlarmClearFailed';
  message: string;
};

export type StorageWriteError = {
  type: 'StorageQuotaExceeded' | 'StorageWriteFailed';
  message: string;
};
