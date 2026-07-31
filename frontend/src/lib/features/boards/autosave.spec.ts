import { describe, expect, it } from 'vitest';
import {
  normalizedCustomName,
  SerialSaveQueue,
  validParallelCount,
  validPreferredCanId,
  validSeriesCount,
} from './autosave';

describe('SerialSaveQueue', () => {
  it('preserves rapid mutation order and flushes through the latest change', async () => {
    const queue = new SerialSaveQueue();
    const completed: number[] = [];
    queue.enqueue(async () => {
      await Promise.resolve();
      completed.push(1);
    });
    queue.enqueue(async () => {
      completed.push(2);
    });
    await queue.flush();
    expect(completed).toEqual([1, 2]);
  });

  it('continues after a rejected save so a newer edit is not lost', async () => {
    const queue = new SerialSaveQueue();
    const completed: number[] = [];
    queue.enqueue(async () => {
      throw new Error('conflict');
    });
    queue.enqueue(async () => {
      completed.push(2);
    });
    await queue.flush();
    expect(completed).toEqual([2]);
  });

  it('validates progressive fields independently and supports clearing a custom name', () => {
    expect(validSeriesCount(undefined)).toBe(false);
    expect(validSeriesCount(18)).toBe(true);
    expect(validSeriesCount(18.5)).toBe(false);
    expect(validParallelCount(2)).toBe(true);
    expect(validParallelCount(0)).toBe(false);
    expect(validPreferredCanId(0)).toBe(true);
    expect(validPreferredCanId(253)).toBe(true);
    expect(validPreferredCanId(254)).toBe(false);
    expect(validPreferredCanId(1.5)).toBe(false);
    expect(validPreferredCanId(undefined)).toBe(false);
    expect(normalizedCustomName('  Trail board ')).toBe('Trail board');
    expect(normalizedCustomName('   ')).toBeNull();
  });
});
