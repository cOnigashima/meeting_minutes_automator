/**
 * レート制限制御インターフェース
 *
 * 責務: Token Bucketアルゴリズムでレート制限（60リクエスト/分）
 * テスト戦略: 完全にモック可能（⭐⭐⭐⭐⭐）
 */

export interface ITokenBucketRateLimiter {
  acquire(): Promise<void>;
  getAvailableTokens(): number;
}
