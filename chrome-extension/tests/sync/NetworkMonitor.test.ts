import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { NetworkMonitor } from '@/sync/NetworkMonitor';

describe('NetworkMonitor', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should return current navigator.onLine state', () => {
    Object.defineProperty(window.navigator, 'onLine', { value: false, configurable: true });
    const monitor = new NetworkMonitor();
    expect(monitor.isOnline()).toBe(false);

    Object.defineProperty(window.navigator, 'onLine', { value: true, configurable: true });
    expect(monitor.isOnline()).toBe(true);
  });

  it('should call callback on online/offline events', () => {
    const monitor = new NetworkMonitor();
    const callback = vi.fn();

    monitor.onStateChange(callback);
    window.dispatchEvent(new Event('online'));
    window.dispatchEvent(new Event('offline'));

    expect(callback).toHaveBeenNthCalledWith(1, true);
    expect(callback).toHaveBeenNthCalledWith(2, false);
  });

  it('should remove listener and stop callbacks', () => {
    const monitor = new NetworkMonitor();
    const callback = vi.fn();

    monitor.onStateChange(callback);
    monitor.removeStateChangeListener();

    window.dispatchEvent(new Event('online'));
    window.dispatchEvent(new Event('offline'));

    expect(callback).not.toHaveBeenCalled();
  });
});
