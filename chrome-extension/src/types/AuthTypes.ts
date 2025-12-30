/**
 * Auth Domain Types
 */

export type AuthTokens = {
  accessToken: string;
  refreshToken: string;
  expiresAt: number; // Unix timestamp (ms)
};

export type TokenResponse = {
  access_token: string;
  refresh_token?: string;
  expires_in: number;
  token_type: string;
};

export type AuthError =
  | { type: 'UserCancelled' }
  | { type: 'NetworkError'; message: string }
  | { type: 'InvalidGrant'; message: string };

export type TokenExpiredError =
  | { type: 'RefreshRequired'; message: string }
  | { type: 'RefreshFailed'; message: string };

export type RefreshError =
  | { type: 'InvalidRefreshToken'; message: string }
  | { type: 'RefreshFailed'; message: string };

export type RevokeError = {
  type: 'RevokeFailed';
  message: string;
};

export type StorageError = {
  type: 'QuotaExceeded' | 'WriteError';
  message: string;
};

export type TokenExchangeError = {
  type: 'InvalidGrant' | 'NetworkError';
  message: string;
};

export type AuthFlowError = {
  type: 'UserCancelled' | 'NetworkError';
  message: string;
};
