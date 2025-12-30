/**
 * Chrome Identity API の抽象化インターフェース
 *
 * 責務: Chrome Identity API の低レベル呼び出しをカプセル化
 * テスト戦略: モックオブジェクトで完全にスタブ可能（⭐⭐⭐）
 *
 * Reference: design-artifacts/interface-contracts.md
 *
 * Note: getAuthToken() を使用。Chromeがトークンをキャッシュし、
 * 有効期限の管理も自動で行う。
 */

import type { Result } from '@/types/Result';
import type { AuthFlowError } from '@/types/AuthTypes';

export interface IChromeIdentityClient {
  /**
   * アクセストークンを取得する（Chrome管理）
   *
   * @param interactive trueの場合、同意画面を表示可能
   * @preconditions manifest.jsonにoauth2セクションが設定済み
   * @postconditions アクセストークンが返される（Chromeがキャッシュ管理）
   * @throws UserCancelledError ユーザーがキャンセル
   * @throws NetworkError ネットワークエラー
   * @returns Result<アクセストークン, AuthFlowError>
   */
  getAccessToken(interactive: boolean): Promise<Result<string, AuthFlowError>>;

  /**
   * キャッシュされたトークンを削除する
   *
   * @param token 削除するトークン
   * @postconditions Chromeのトークンキャッシュから削除される
   */
  removeCachedToken(token: string): Promise<void>;
}
