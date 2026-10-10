import { expect, it, vi } from 'vitest';
import { MapSeriesLoader } from './map-series';
import type { SessionReviewController } from '$lib/backend/session-controller';
import { recordSessionDataDecodedBytes, type SessionSeriesResult } from '$lib/backend/session-data';
const ranges = [
  { startElapsedMs: 0, endElapsedMs: 100 },
  { startElapsedMs: 500, endElapsedMs: 600 },
  { startElapsedMs: 900, endElapsedMs: 1000 },
];
const response = (start: number, end: number): SessionSeriesResult =>
  ({
    kind: 'series',
    range: { startElapsedMs: start, endElapsedMs: end },
    fields: [{ name: 'speedKph', source: 'stable' }],
    columns: { speedKph: { elapsedMs: [start, end], sequence: [start, end], values: [1, 2], generations: [0, 0] } },
  }) as unknown as SessionSeriesResult;
it('bounds concurrency across groups and returns all intervals with separate generations', async () => {
  let running = 0,
    peak = 0;
  const loadSeries = vi.fn(async (_fields: string[], start: number, end: number) => {
    running++;
    peak = Math.max(peak, running);
    await new Promise((resolve) => setTimeout(resolve, 1));
    running--;
    return response(start, end);
  });
  const loader = new MapSeriesLoader({ loadSeries, cancelPending: vi.fn() } as unknown as SessionReviewController);
  const [result] = await Promise.all([
    loader.load(['speedKph'], ranges, 300, null),
    loader.load(['other'], ranges, 300, null),
  ]);
  expect(peak).toBe(2);
  expect(result.columns.speedKph!.elapsedMs).toEqual([0, 100, 500, 600, 900, 1000]);
  expect(result.columns.speedKph!.generations).toEqual([0, 0, 1, 1, 2, 2]);
  expect(loadSeries.mock.calls).toHaveLength(6);
});
it('rejects late completions after cancellation and can load again', async () => {
  let release!: () => void;
  const gate = new Promise<void>((resolve) => (release = resolve));
  const loadSeries = vi.fn(async (_fields: string[], start: number, end: number) => {
    await gate;
    return response(start, end);
  });
  const cancelPending = vi.fn();
  const loader = new MapSeriesLoader({ loadSeries, cancelPending } as unknown as SessionReviewController);
  const pending = loader.load(['speedKph'], ranges, 300, null);
  loader.cancel();
  release();
  await expect(pending).rejects.toMatchObject({ name: 'AbortError' });
  expect(cancelPending).toHaveBeenCalledOnce();
  expect((await loader.load(['speedKph'], ranges, 300, null)).columns.speedKph!.values).toHaveLength(6);
});
it('fails atomically on one interval failure and rejects unsupported interval count', async () => {
  const loader = new MapSeriesLoader({
    loadSeries: async (_fields: string[], start: number, end: number) => {
      if (start === 500) throw new Error('offline');
      return response(start, end);
    },
    cancelPending: vi.fn(),
  } as unknown as SessionReviewController);
  await expect(loader.load(['speedKph'], ranges, 300, null)).rejects.toThrow('offline');
  await expect(loader.load(['speedKph'], Array(129).fill(ranges[0]), 300, null)).rejects.toThrow('interval count');
});

it('limits cumulative decoded results and releases failed reservations for retry', async () => {
  let large = true;
  const loader = new MapSeriesLoader({
    loadSeries: async (_fields: string[], start: number, end: number) => {
      const result = response(start, end);
      recordSessionDataDecodedBytes(result, large ? 9 * 1024 * 1024 : 100);
      return result;
    },
    cancelPending: vi.fn(),
  } as unknown as SessionReviewController);
  await expect(loader.load(['speedKph'], ranges, 300, null)).rejects.toThrow('review limit');
  large = false;
  expect((await loader.load(['speedKph'], ranges, 300, null)).columns.speedKph!.values).toHaveLength(6);
});
