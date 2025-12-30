import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { StorageMonitor } from '@/sync/StorageMonitor';

const alarmListeners: Array<(alarm: chrome.alarms.Alarm) => void | Promise<void>> = [];

const mockChrome = {
  storage: {
    local: {
      getBytesInUse: vi.fn(async () => 0),
      QUOTA_BYTES: 1000,
    },
  },
  alarms: {
    create: vi.fn(async () => undefined),
    clear: vi.fn(async () => true),
    onAlarm: {
      addListener: vi.fn((listener: (alarm: chrome.alarms.Alarm) => void) => {
        alarmListeners.push(listener);
      }),
      removeListener: vi.fn((listener: (alarm: chrome.alarms.Alarm) => void) => {
        const index = alarmListeners.indexOf(listener);
        if (index >= 0) alarmListeners.splice(index, 1);
      }),
    },
  },
};

vi.stubGlobal('chrome', mockChrome);

describe('StorageMonitor', () => {
  beforeEach(() => {
    alarmListeners.splice(0, alarmListeners.length);
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should return usage percentage', async () => {
    mockChrome.storage.local.getBytesInUse.mockResolvedValueOnce(500);

    const monitor = new StorageMonitor();
    const usage = await monitor.getUsagePercentage();

    expect(usage).toBe(50);
  });

  it('should start monitoring and respond to alarm', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    mockChrome.storage.local.getBytesInUse.mockResolvedValueOnce(950);

    const monitor = new StorageMonitor();
    await monitor.startMonitoring();

    expect(mockChrome.alarms.create).toHaveBeenCalledWith(
      'storage_monitor_alarm',
      expect.objectContaining({ periodInMinutes: expect.any(Number) })
    );
    expect(mockChrome.alarms.onAlarm.addListener).toHaveBeenCalledTimes(1);
    expect(alarmListeners).toHaveLength(1);

    await alarmListeners[0]({ name: 'storage_monitor_alarm' } as chrome.alarms.Alarm);

    expect(warnSpy).toHaveBeenCalled();
  });

  it('should stop monitoring and remove listener', async () => {
    const monitor = new StorageMonitor();
    await monitor.startMonitoring();
    await monitor.stopMonitoring();

    expect(mockChrome.alarms.clear).toHaveBeenCalledWith('storage_monitor_alarm');
    expect(mockChrome.alarms.onAlarm.removeListener).toHaveBeenCalled();
  });
});
