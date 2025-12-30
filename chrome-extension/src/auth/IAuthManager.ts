/**
 * OAuth 2.0認証フロー統合インターフェース
 *
 * 責務: 認証フロー実行とトークンライフサイクル管理
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 *
 * Reference: design-artifacts/interface-contracts.md
 */

import type { Result } from '@/types/Result';
import type { AuthTokens, AuthError, TokenExpiredError, RevokeError } from '@/types/AuthTypes';

export interface IAuthManager {
  /**
   * OAuth 2.0認証フローを開始し、トークンを取得
   *
   * @preconditions chrome.identity APIが利用可能
   * @postconditions トークンがTokenStoreに保存される
   * @throws AuthError トークン取得失敗、ユーザーキャンセル
   */
  initiateAuth(): Promise<Result<AuthTokens, AuthError>>;

  /**
   * アクセストークンを取得（期限切れ時は自動リフレッシュ）
   *
   * @preconditions TokenStoreにリフレッシュトークンが保存されている
   * @postconditions 有効なアクセストークンが返される
   * @throws TokenExpiredError リフレッシュトークンが無効、または一時的に更新失敗
   */
  getAccessToken(): Promise<Result<string, TokenExpiredError>>;

  /**
   * トークンを無効化し、TokenStoreから削除
   *
   * @postconditions TokenStoreが空になる
   * @throws RevokeError トークン無効化失敗（ベストエフォート）
   */
  revokeToken(): Promise<Result<void, RevokeError>>;
}
