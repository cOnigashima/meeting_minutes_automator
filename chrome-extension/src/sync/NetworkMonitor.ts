/**
 * ネットワーク監視実装
 *
 * Implementation: Phase 3
 */

import type { INetworkMonitor } from './INetworkMonitor';

export class NetworkMonitor implements INetworkMonitor {
  private callback: ((isOnline: boolean) => void) | null = null;
  private readonly eventTarget: EventTarget;
  private readonly onlineListener = () => {
    this.callback?.(true);
  };
  private readonly offlineListener = () => {
    this.callback?.(false);
  };

  constructor() {
    this.eventTarget = (typeof window !== 'undefined' ? window : globalThis) as EventTarget;
  }

  isOnline(): boolean {
    return typeof navigator !== 'undefined' ? navigator.onLine : true;
  }

  onStateChange(callback: (isOnline: boolean) => void): void {
    this.removeStateChangeListener();
    this.callback = callback;
    this.eventTarget.addEventListener('online', this.onlineListener);
    this.eventTarget.addEventListener('offline', this.offlineListener);
  }

  removeStateChangeListener(): void {
    if (!this.callback) return;
    this.eventTarget.removeEventListener('online', this.onlineListener);
    this.eventTarget.removeEventListener('offline', this.offlineListener);
    this.callback = null;
  }
}
