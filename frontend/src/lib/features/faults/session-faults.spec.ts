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

it('loads timeout activation and clear separately without fabricating firmware codes or times', async () => {
  const result = page([
    [1, 'imuTimeoutHistory', '', null, null],
    [2, 'imuTimeoutHistory', '', null, null],
    [3, 'firmwareFaultHistory', 'driver', 3, 4900],
  ]);
  result.page.timeouts = [
    { active: true, sourceTicks: 100, occurredElapsedMs: 4800 },
    { active: false, sourceTicks: 101, occurredElapsedMs: null },
    null,
  ];
  const faults = await loadFirmwareFaults({ loadEvents: vi.fn().mockResolvedValue(result) }, metadata);
  expect(faults.map((f) => [f.reason, f.code, f.elapsedMs])).toEqual([
    ['imuTimeout', null, 4800],
    ['driver', 3, 4900],
    ['imuTimeoutCleared', null, null],
  ]);
  expect(faultLabel('imuTimeout', null)).toBe('IMU timeout');
  expect(faultLabel('imuTimeoutCleared', null)).toBe('IMU timeout cleared');
});

it('keeps sampled activation and clear across pages, ignoring the healthy baseline and duplicate stop', async () => {
  const first = page([[1, 'imuTimeoutActive', 'cleared', null, null]], 'next');
  first.page.elapsedMs = [0];
  const second = page([
    [2, 'stopCondition', 'imuTimeout', null, null],
    [3, 'imuTimeoutActive', 'active', null, null],
    [4, 'imuTimeoutActive', 'cleared', null, null],
  ]);
  second.page.elapsedMs = [1000, 1000, 2000];
  const loadEvents = vi.fn().mockResolvedValueOnce(first).mockResolvedValueOnce(second);
  expect((await loadFirmwareFaults({ loadEvents }, metadata)).map((row) => [row.reason, row.elapsedMs])).toEqual([
    ['imuTimeout', 1000],
    ['imuTimeoutCleared', 2000],
  ]);
});

it('prefers recovered timeout records over sampled active and clear transitions', async () => {
  const result = page([
    [1, 'imuTimeoutActive', 'active', null, null],
    [2, 'imuTimeoutActive', 'cleared', null, null],
    [3, 'imuTimeoutHistory', '', null, null],
    [4, 'imuTimeoutHistory', '', null, null],
  ]);
  result.page.elapsedMs = [1000, 2000, 3000, 3000];
  result.page.timeouts = [
    null,
    null,
    { active: true, sourceTicks: 10, occurredElapsedMs: 900 },
    { active: false, sourceTicks: 20, occurredElapsedMs: 1900 },
  ];
  expect(
    (await loadFirmwareFaults({ loadEvents: vi.fn().mockResolvedValue(result) }, metadata)).map((row) => row.elapsedMs)
  ).toEqual([900, 1900]);
});

it('does not present initial inactive state as a fault', async () => {
  expect(
    await loadFirmwareFaults(
      { loadEvents: vi.fn().mockResolvedValue(page([[1, 'imuTimeoutActive', 'cleared', null, null]])) },
      metadata
    )
  ).toEqual([]);
});

it('keeps a retained stop undated when first observed well after the timeout', async () => {
  const result = page([[1, 'stopCondition', 'imuTimeout', null, null]]);
  result.page.elapsedMs = [30_000];
  const rows = await loadFirmwareFaults({ loadEvents: vi.fn().mockResolvedValue(result) }, metadata);
  expect(rows).toEqual([{ sequence: 1, reason: 'imuTimeout', code: null, elapsedMs: null, observedElapsedMs: 30_000 }]);
  expect(faultTime(rows[0]!.elapsedMs)).toBe('Time unavailable');
});

it('replaces delayed sampled activation and clearance across pages with recovered occurrences', async () => {
  const first = page(
    [
      [1, 'imuTimeoutActive', 'cleared', null, null],
      [2, 'imuTimeoutActive', 'active', null, null],
      [3, 'imuTimeoutActive', 'cleared', null, null],
    ],
    'history'
  );
  first.page.elapsedMs = [0, 10_000, 20_000];
  const second = page([
    [4, 'imuTimeoutHistory', '', null, null],
    [5, 'imuTimeoutHistory', '', null, null],
  ]);
  second.page.elapsedMs = [30_000, 30_000];
  second.page.timeouts = [
    { active: true, sourceTicks: 100, occurredElapsedMs: 5000 },
    { active: false, sourceTicks: 200, occurredElapsedMs: 15_000 },
  ];
  const rows = await loadFirmwareFaults(
    { loadEvents: vi.fn().mockResolvedValueOnce(first).mockResolvedValueOnce(second) },
    metadata
  );
  expect(rows.map((row) => [row.reason, row.elapsedMs])).toEqual([
    ['imuTimeout', 5000],
    ['imuTimeoutCleared', 15_000],
  ]);
});

it('preserves a separate sampled episode outside recovered transition windows', async () => {
  const result = page([
    [1, 'imuTimeoutActive', 'active', null, null],
    [2, 'imuTimeoutActive', 'cleared', null, null],
    [3, 'imuTimeoutActive', 'active', null, null],
    [4, 'imuTimeoutActive', 'cleared', null, null],
    [5, 'imuTimeoutHistory', '', null, null],
    [6, 'imuTimeoutHistory', '', null, null],
  ]);
  result.page.elapsedMs = [10_000, 20_000, 30_000, 40_000, 50_000, 50_000];
  result.page.timeouts = [
    null,
    null,
    null,
    null,
    { active: true, sourceTicks: 100, occurredElapsedMs: 5000 },
    { active: false, sourceTicks: 200, occurredElapsedMs: 15_000 },
  ];
  const rows = await loadFirmwareFaults({ loadEvents: vi.fn().mockResolvedValue(result) }, metadata);
  expect(rows.map((row) => row.elapsedMs)).toEqual([5000, 15_000, 30_000, 40_000]);
});

it('matches undated recovered transitions by order without assigning them sampled times', async () => {
  const result = page([
    [1, 'imuTimeoutActive', 'active', null, null],
    [2, 'imuTimeoutActive', 'cleared', null, null],
    [3, 'imuTimeoutHistory', '', null, null],
    [4, 'imuTimeoutHistory', '', null, null],
  ]);
  result.page.elapsedMs = [10_000, 20_000, 30_000, 30_000];
  result.page.timeouts = [
    null,
    null,
    { active: true, sourceTicks: 100, occurredElapsedMs: null },
    { active: false, sourceTicks: 200, occurredElapsedMs: null },
  ];
  const rows = await loadFirmwareFaults({ loadEvents: vi.fn().mockResolvedValue(result) }, metadata);
  expect(rows.map((row) => [row.reason, row.elapsedMs, row.observedElapsedMs])).toEqual([
    ['imuTimeout', null, 30_000],
    ['imuTimeoutCleared', null, 30_000],
  ]);
});

it('preserves a retained stop when a later sampled activation may be a separate episode', async () => {
  const result = page([
    [1, 'stopCondition', 'imuTimeout', null, null],
    [2, 'imuTimeoutActive', 'active', null, null],
    [3, 'imuTimeoutActive', 'cleared', null, null],
  ]);
  result.page.elapsedMs = [5000, 10_000, 20_000];
  const rows = await loadFirmwareFaults({ loadEvents: vi.fn().mockResolvedValue(result) }, metadata);
  expect(rows.map((row) => row.elapsedMs)).toEqual([null, 10_000, 20_000]);
});

it('does not hide a sampled active episode behind an older recovered episode that already cleared', async () => {
  const result = page([
    [1, 'imuTimeoutActive', 'active', null, null],
    [2, 'imuTimeoutHistory', '', null, null],
    [3, 'imuTimeoutHistory', '', null, null],
  ]);
  result.page.elapsedMs = [10_000, 20_000, 20_000];
  result.page.timeouts = [
    null,
    { active: true, sourceTicks: 100, occurredElapsedMs: 1000 },
    { active: false, sourceTicks: 200, occurredElapsedMs: 2000 },
  ];
  const rows = await loadFirmwareFaults({ loadEvents: vi.fn().mockResolvedValue(result) }, metadata);
  expect(rows.map((row) => row.elapsedMs)).toEqual([1000, 2000, 10_000]);
});

it.each([false, true])('does not fabricate clearance after a retained stop (paged: %s)', async (paged) => {
  const stop = [1, 'stopCondition', 'imuTimeout', null, null] as const;
  const baseline = [2, 'imuTimeoutActive', 'cleared', null, null] as const;
  const first = page(paged ? [[...stop]] : [[...stop], [...baseline]], paged ? 'baseline' : null);
  first.page.elapsedMs = paged ? [1000] : [1000, 1100];
  const second = page([[...baseline]]);
  second.page.elapsedMs = [1100];
  const loadEvents = vi.fn().mockResolvedValueOnce(first).mockResolvedValueOnce(second);
  expect(await loadFirmwareFaults({ loadEvents }, metadata)).toEqual([
    { sequence: 1, reason: 'imuTimeout', code: null, elapsedMs: null, observedElapsedMs: 1000 },
  ]);
});

it('keeps actual activation and clearance after a retained stop and healthy baseline', async () => {
  const result = page([
    [1, 'stopCondition', 'imuTimeout', null, null],
    [2, 'imuTimeoutActive', 'cleared', null, null],
    [3, 'imuTimeoutActive', 'active', null, null],
    [4, 'imuTimeoutActive', 'cleared', null, null],
  ]);
  result.page.elapsedMs = [1000, 1100, 2000, 3000];
  const rows = await loadFirmwareFaults({ loadEvents: vi.fn().mockResolvedValue(result) }, metadata);
  expect(rows.map((row) => [row.reason, row.elapsedMs])).toEqual([
    ['imuTimeout', null],
    ['imuTimeout', 2000],
    ['imuTimeoutCleared', 3000],
  ]);
});
