import { describe, it, expect, vi } from 'vitest';
import { BufferingManager } from '@/sync/BufferingManager';
import type { TranscriptionMessage } from '@/types/SyncTypes';

describe('BufferingManager', () => {
  const message = (text: string): TranscriptionMessage => ({
    text,
    timestamp: Date.now(),
    isPartial: false,
  });

  it('should flush buffered messages via handler and clear buffer', async () => {
    const handler = vi.fn().mockResolvedValue(undefined);
    const manager = new BufferingManager(handler);

    manager.buffer(message('first'));
    manager.buffer(message('second'));

    await manager.flush();

    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith([
      expect.objectContaining({ text: 'first' }),
      expect.objectContaining({ text: 'second' }),
    ]);

    await manager.flush();
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('should keep messages when handler fails', async () => {
    const handler = vi.fn(async () => {
      throw new Error('flush failed');
    });
    const manager = new BufferingManager(handler);

    manager.buffer(message('first'));
    manager.buffer(message('second'));

    await expect(manager.flush()).rejects.toThrow('flush failed');
    expect(handler).toHaveBeenCalledTimes(1);

    // Second flush attempts again with same messages
    await expect(manager.flush()).rejects.toThrow('flush failed');
    expect(handler).toHaveBeenCalledTimes(2);
  });

  it('should not throw when no handler is provided', async () => {
    const manager = new BufferingManager();
    manager.buffer(message('hello'));

    await expect(manager.flush()).resolves.toBeUndefined();
    await expect(manager.flush()).resolves.toBeUndefined();
  });
});
