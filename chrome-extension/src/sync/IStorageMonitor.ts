/**
 * ストレージ監視インターフェース
 *
 * 責務: chrome.storage.localの使用状況監視、警告通知
 * テスト戦略: 依存性注入で容易にモック可能（⭐⭐⭐⭐）
 */

export interface IStorageMonitor {
  getUsagePercentage(): Promise<number>;
  startMonitoring(): Promise<void>;
  stopMonitoring(): Promise<void>;
}
