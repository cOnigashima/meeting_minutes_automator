/**
 * トークンストレージの抽象化インターフェース
 *
 * 責務: トークンの永続化のみ（検証ロジックは含まない）
 * テスト戦略: インメモリ実装で完全にモック可能（⭐⭐⭐⭐⭐）
 *
 * Reference: design-artifacts/interface-contracts.md
 */

import type { Result } from '@/types/Result';
import type { AuthTokens, StorageError } from '@/types/AuthTypes';

export interface ITokenStore {
  /**
   * トークンを保存する
   *
   * @preconditions token が有効な AuthToken
   * @postconditions chrome.storage.local に保存される
   * @throws StorageFullError ストレージ上限到達
   */
  save(token: AuthTokens): Promise<Result<void, StorageError>>;

  /**
   * トークンを読み込む
   *
   * @preconditions なし
   * @postconditions トークンが存在すれば返される
   */
  load(): Promise<AuthTokens | null>;

  /**
   * トークンを削除する
   *
   * @preconditions なし
   * @postconditions chrome.storage.local からトークンが削除される
   */
  remove(): Promise<void>;
}
