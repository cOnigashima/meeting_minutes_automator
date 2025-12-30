/**
 * ストレージ監視実装
 *
 * Implementation: Phase 3
 */

import type { IStorageMonitor } from './IStorageMonitor';

export class StorageMonitor implements IStorageMonitor {
  private readonly ALARM_NAME = 'storage_monitor_alarm';
  private readonly CHECK_INTERVAL_MINUTES = 5;
  private readonly WARNING_THRESHOLD_PERCENT = 90;
  private readonly alarmListener = async (alarm: chrome.alarms.Alarm) => {
    if (alarm.name !== this.ALARM_NAME) return;
    const usage = await this.getUsagePercentage();
    if (usage >= this.WARNING_THRESHOLD_PERCENT) {
      console.warn(`[StorageMonitor] Usage at ${usage.toFixed(1)}%`);
    }
  };

  async getUsagePercentage(): Promise<number> {
    const bytesInUse = await chrome.storage.local.getBytesInUse(null);
    const quota = chrome.storage.local.QUOTA_BYTES || 1;
    return Math.min(100, (bytesInUse / quota) * 100);
  }

  async startMonitoring(): Promise<void> {
    chrome.alarms.onAlarm.removeListener(this.alarmListener);
    chrome.alarms.onAlarm.addListener(this.alarmListener);
    await chrome.alarms.create(this.ALARM_NAME, {
      periodInMinutes: this.CHECK_INTERVAL_MINUTES,
    });
  }

  async stopMonitoring(): Promise<void> {
    await chrome.alarms.clear(this.ALARM_NAME);
    chrome.alarms.onAlarm.removeListener(this.alarmListener);
  }
}
