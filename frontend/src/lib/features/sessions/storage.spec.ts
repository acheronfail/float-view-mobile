import { describe, expect, it } from 'vitest';
import type { SavedDevice } from '$lib/backend/contract';
import { formatStorageBytes, totalSessionStorageBytes } from './storage';

const board = (bytes: number | null) => ({ summary: { totalStorageBytes: bytes } }) as SavedDevice;

describe('session storage', () => {
  it('totals all boards and preserves unavailable measurements', () => {
    expect(totalSessionStorageBytes([])).toBe(0);
    expect(totalSessionStorageBytes([board(123), board(456)])).toBe(579);
    expect(totalSessionStorageBytes([board(123), board(null)])).toBeNull();
  });

  it('formats empty, small and large totals consistently with prune estimates', () => {
    expect(formatStorageBytes(0)).toBe('0 B');
    expect(formatStorageBytes(512)).toBe('512 B');
    expect(formatStorageBytes(2048)).toBe('2.0 KB');
    expect(formatStorageBytes(1024 ** 2)).toBe('1.0 MB');
    expect(formatStorageBytes(1024 ** 3)).toBe('1.0 GB');
    expect(formatStorageBytes(1024 ** 4)).toBe('1.0 TB');
  });
});
