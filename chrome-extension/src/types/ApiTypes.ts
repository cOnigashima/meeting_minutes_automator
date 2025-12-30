/**
 * API Domain Types
 */

export type ApiError = {
  code: number; // HTTP status code
  message: string;
  status?: string; // Google API status (e.g., "INVALID_ARGUMENT")
};

export type NotFoundError = {
  type: 'NotFound';
  message: string;
};

export type MaxRetriesExceededError = {
  type: 'MaxRetriesExceeded';
  message: string;
  retriesAttempted: number;
};

export type ConflictError = {
  type: 'Conflict';
  message: string;
  currentRevisionId: string;
  attemptedRevisionId: string;
};
