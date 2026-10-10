import { expect, it, vi } from 'vitest';
import { loadLocationTrack } from './location-track';
import type { SessionMetadata } from '$lib/backend/contract';
import { recordSessionDataDecodedBytes, type SessionLocationsResult } from '$lib/backend/session-data';
const metadata = { location: { available: true }, summary: { totalElapsedMs: 100_000 } } as SessionMetadata;
const result = (cursor: string | null): SessionLocationsResult =>
  ({ kind: 'locations', points: [], gaps: [], nextCursor: cursor }) as unknown as SessionLocationsResult;
it('exhausts more than four pages', async () => {
  let count = 0;
  const loadLocations = vi.fn(async () => result(++count < 6 ? String(count) : null));
  expect(await loadLocationTrack({ loadLocations }, metadata)).toEqual({ points: [], gaps: [] });
  expect(count).toBe(6);
});
it('rejects repeated cursors and page limits', async () => {
  await expect(loadLocationTrack({ loadLocations: async () => result('same') }, metadata)).rejects.toThrow(
    'did not advance'
  );
  let count = 0;
  await expect(loadLocationTrack({ loadLocations: async () => result(String(++count)) }, metadata)).rejects.toThrow(
    'page limit'
  );
  expect(count).toBe(128);
});
it('does not publish a partial prefix after transport failure', async () => {
  const loadLocations = vi.fn().mockResolvedValueOnce(result('next')).mockRejectedValue(new Error('offline'));
  await expect(loadLocationTrack({ loadLocations }, metadata)).rejects.toThrow('offline');
});
it('bounds cumulative track data', async () => {
  const oversized = result(null);
  oversized.points = Array(100001).fill({ elapsedMs: 0 });
  await expect(loadLocationTrack({ loadLocations: async () => oversized }, metadata)).rejects.toThrow('review limits');
});

it('rejects decoded byte overflow even with few rows', async () => {
  const value = result(null);
  recordSessionDataDecodedBytes(value, 17 * 1024 * 1024);
  await expect(loadLocationTrack({ loadLocations: async () => value }, metadata)).rejects.toThrow('review limits');
});
