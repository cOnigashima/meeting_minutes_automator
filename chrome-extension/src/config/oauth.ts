/**
 * OAuth 2.0 設定
 *
 * 責務: OAuth設定値の一元管理
 * ファイルパス: chrome-extension/src/config/oauth.ts
 *
 * Implementation: Phase 1
 *
 * Note: 本番環境では Chrome App OAuth type を使用するため、
 * client_secret は不要（PKCE のみで認証可能）
 *
 * Setup:
 * 1. Google Cloud Console で OAuth クライアントを作成
 * 2. 以下の CLIENT_ID を実際の値に置き換える
 */

export const OAUTH_CONFIG = {
  /**
   * Google OAuth 2.0 Client ID
   *
   * Chrome App タイプを使用（PKCE のみ、client_secret 不要）
   *
   * 開発・本番ともに Chrome Extension 用の OAuth クライアントを使用。
   * Google Cloud Console > APIs & Services > Credentials で作成。
   */
  clientId: '242619749099-irha686ovagpbiieecpf06t3ph4a9ois.apps.googleusercontent.com',

  /**
   * OAuth 2.0 スコープ
   */
  scopes: [
    'https://www.googleapis.com/auth/documents',
    'https://www.googleapis.com/auth/drive.file',
  ],
} as const;
