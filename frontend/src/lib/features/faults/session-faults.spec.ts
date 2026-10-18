import { expect, it, vi } from 'vitest';
import { loadFirmwareFaults } from './session-faults';
import { faultLabel, faultTime } from './faults';
import type { SessionMetadata } from '$lib/backend/contract';
import type { SessionEventsResult } from '$lib/backend/session-data';

const metadata = { snapshot: { committedElapsedMs: 60_000 } } as SessionMetadata;
function page(
  rows: [number, string, string, number | null, number | null][],
  nextCursor: string | null = null
): SessionEventsResult {
  return {
    kind: 'events',
    page: {
      sequence: rows.map((r) => r[0]),
      elapsedMs: rows.map(() => 5000),
      sourceSampleSequence: rows.map(() => 0),
      kinds: rows.map((r) => r[1]),
      values: rows.map((r) => r[2]),
      faults: rows.map((r) => (r[3] === null ? null : { rawCode: r[3], occurredElapsedMs: r[4] })),
    },
    nextCursor,
  } as SessionEventsResult;
}

it('loads every page, keeps unknown times and uses recovered fault times over duplicate state transitions', async () => {
  const loadEvents = vi
    .fn()
    .mockResolvedValueOnce(
      page(
        [
          [1, 'firmwareFault', 'absoluteOverCurrent', null, null],
          [2, 'firmwareFaultHistory', 'absoluteOverCurrent', 4, 4800],
          [3, 'firmwareFaultHistory', 'none', 0, 4900],
        ],
        'page-2'
      )
    )
    .mockResolvedValueOnce(page([[4, 'firmwareFaultHistory', 'unknown', 255, null]]));
  const faults = await loadFirmwareFaults({ loadEvents }, metadata);
  expect(faults).toHaveLength(2);
  expect(faults[0]).toMatchObject({ code: 4, elapsedMs: 4800 });
  expect(faults[1]).toMatchObject({ code: 255, elapsedMs: null });
  expect(loadEvents.mock.calls[1]![0]).toMatchObject({ cursor: 'page-2' });
});

it('retains older sampled faults and leaves unaligned old history unplaced', async () => {
  const loadEvents = vi.fn().mockResolvedValue(
    page([
      [1, 'firmwareFault', 'driver', null, null],
      [2, 'firmwareFaultHistory', 'underVoltage', 2, null],
    ])
  );
  const faults = await loadFirmwareFaults({ loadEvents }, metadata);
  expect(faults.map((f) => f.elapsedMs)).toEqual([5000, null]);
});

it('rejects incomplete pagination instead of presenting a partial history', async () => {
  const loadEvents = vi.fn().mockResolvedValue(page([], 'same-cursor'));
  await expect(loadFirmwareFaults({ loadEvents }, metadata)).rejects.toThrow('completely');
});

it('uses terse labels and never interprets arbitrary event text as markup', () => {
  expect(faultLabel('absoluteOverCurrent', 4)).toBe('Absolute overcurrent');
  expect(faultLabel('unknown', 255)).toBe('Controller fault 255');
  expect(faultLabel('<img onerror=alert(1)>', null)).toBe('Unknown controller fault');
  expect(faultTime(65_000)).toBe('≈ 1:05');
  expect(faultTime(null)).toBe('Time unavailable');
});
