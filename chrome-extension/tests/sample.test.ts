import { describe, it, expect } from 'vitest';

describe('Sample Test', () => {
  it('should pass basic arithmetic', () => {
    expect(1 + 1).toBe(2);
  });

  it('should validate array operations', () => {
    const arr = [1, 2, 3];
    expect(arr).toHaveLength(3);
    expect(arr[0]).toBe(1);
  });

  it('should handle async operations', async () => {
    const result = await Promise.resolve(42);
    expect(result).toBe(42);
  });
});
