/**
 * ネットワーク監視インターフェース
 *
 * 責務: オンライン/オフライン検知、状態変更通知
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */

export interface INetworkMonitor {
  isOnline(): boolean;
  onStateChange(callback: (isOnline: boolean) => void): void;
  removeStateChangeListener(): void;
}
