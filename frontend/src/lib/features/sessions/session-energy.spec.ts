import { describe, expect, it, vi } from 'vitest';
import type { SessionMetadata } from '$lib/backend/contract';
import type { SessionReviewController } from '$lib/backend/session-controller';
import type { SessionSamplesRequest } from '$lib/backend/session-data';
import { ENERGY_COUNTERS, loadSessionEnergy } from './session-energy';

type Row = [number, number | null, number | null];
function fixture(rows: Row[], pageSize = 2) {
  const metadata = {
    summary: { sessionId: 'session', totalElapsedMs: rows.at(-1)?.[0] ?? 0 },
    fields: ENERGY_COUNTERS.map((name) => ({ name, source: 'stable' })),
    storedSampleCount: rows.length,
    prune: { targetResolutionSeconds: null },
    snapshot: {},
  } as SessionMetadata;
  const loadSamples = vi.fn(async (request: SessionSamplesRequest) => {
    const filtered = rows
      .map((row, sequence) => ({ row, sequence }))
      .filter(({ row }) => row[0] <= request.range.endElapsedMs);
    const start = Number(request.cursor ?? 0);
    const page = filtered.slice(start, start + pageSize);
    return {
      sequence: page.map(({ sequence }) => sequence),
      elapsedMs: page.map(({ row }) => row[0]),
      values: Object.fromEntries(ENERGY_COUNTERS.map((name, i) => [name, page.map(({ row }) => row[i + 1])])),
      nextCursor: start + page.length < filtered.length ? String(start + page.length) : null,
    };
  });
  return { metadata, loadSamples, controller: { loadSamples } as unknown as SessionReviewController };
}

describe('session energy', () => {
  it('subtracts a shared initial baseline across pages without changing stored counters', async () => {
    const rows: Row[] = [
      [0, 800, 100],
      [1000, 820, 105],
      [2000, 850, 110],
      [3000, 850, 120],
    ];
    const { controller, metadata, loadSamples } = fixture(rows);
    const result = await loadSessionEnergy(
      controller,
      metadata,
      [{ startElapsedMs: 0, endElapsedMs: 3000 }],
      100,
      null
    );
    expect(result.columns.wattHoursDrawnWh!.values).toEqual([0, 20, 50, 50]);
    expect(result.columns.wattHoursChargedWh!.values).toEqual([0, 5, 10, 20]);
    expect(result.columns.netEnergyWh!.values).toEqual([0, 15, 40, 30]);
    expect(loadSamples.mock.calls[1]![0].cursor).toBe('2');
    expect(rows[0]).toEqual([0, 800, 100]);
  });

  it('retains net extrema before reduction and allows net recovery', async () => {
    const { controller, metadata } = fixture([
      [0, 100, 20],
      [1000, 110, 20],
      [2000, 111, 40],
      [3000, 120, 40],
    ]);
    const result = await loadSessionEnergy(controller, metadata, [{ startElapsedMs: 0, endElapsedMs: 3000 }], 1, null);
    expect(result.columns.netEnergyWh!.values).toEqual([0, 10, -9, 0]);
  });

  it('keeps the session baseline when zoomed or filtered and breaks disjoint map intervals', async () => {
    const { controller, metadata, loadSamples } = fixture([
      [0, 100, 20],
      [1000, 110, 20],
      [2000, 120, 21],
      [3000, 130, 22],
    ]);
    const result = await loadSessionEnergy(
      controller,
      metadata,
      [
        { startElapsedMs: 1000, endElapsedMs: 1500 },
        { startElapsedMs: 2500, endElapsedMs: 3000 },
      ],
      100,
      null
    );
    expect(loadSamples.mock.calls[0]![0].range).toEqual({ startElapsedMs: 0, endElapsedMs: 3000 });
    expect(result.columns.netEnergyWh!.values).toEqual([10, 28]);
    expect(new Set(result.columns.netEnergyWh!.generations).size).toBe(2);
  });

  it('waits for matching available counters and breaks traces at missing readings and time gaps', async () => {
    const { controller, metadata } = fixture([
      [0, 100, null],
      [1000, 110, 20],
      [2000, null, 21],
      [3000, 130, 22],
      [10000, 140, 22],
    ]);
    const result = await loadSessionEnergy(
      controller,
      metadata,
      [{ startElapsedMs: 0, endElapsedMs: 10000 }],
      100,
      null
    );
    expect(result.columns.netEnergyWh!.values).toEqual([0, 18, 28]);
    expect(new Set(result.columns.netEnergyWh!.generations).size).toBe(3);
  });

  it('stops affected totals after a counter reset, even when the counter later exceeds its old baseline', async () => {
    const { controller, metadata } = fixture([
      [0, 100, 20],
      [1000, 120, 22],
      [2000, 0, 23],
      [3000, 150, 24],
    ]);
    const result = await loadSessionEnergy(
      controller,
      metadata,
      [{ startElapsedMs: 0, endElapsedMs: 3000 }],
      100,
      null
    );
    expect(result.columns.wattHoursDrawnWh!.values).toEqual([0, 20]);
    expect(result.columns.netEnergyWh!.values).toEqual([0, 18]);
    expect(result.columns.wattHoursChargedWh!.values).toEqual([0, 2, 3, 4]);
  });

  it('supports a lone counter without inventing net consumption', async () => {
    const { controller, metadata } = fixture([
      [0, 100, null],
      [1000, 120, null],
    ]);
    metadata.fields = [metadata.fields[0]!];
    const result = await loadSessionEnergy(
      controller,
      metadata,
      [{ startElapsedMs: 0, endElapsedMs: 1000 }],
      100,
      null
    );
    expect(result.columns.wattHoursDrawnWh!.values).toEqual([0, 20]);
    expect(result.columns.netEnergyWh).toBeUndefined();
  });

  it('bounds output and rejects oversized or nonadvancing history', async () => {
    const { controller, metadata, loadSamples } = fixture(
      Array.from({ length: 1000 }, (_, i) => [i, 100 + i, 20 + i / 2]),
      100
    );
    const result = await loadSessionEnergy(controller, metadata, [{ startElapsedMs: 0, endElapsedMs: 1000 }], 10, null);
    expect(Object.values(result.columns).every((column) => column.values.length <= 40)).toBe(true);
    await expect(
      loadSessionEnergy(
        controller,
        { ...metadata, storedSampleCount: 1_000_001 },
        [{ startElapsedMs: 0, endElapsedMs: 1000 }],
        10,
        null
      )
    ).rejects.toThrow('too large');
    loadSamples.mockResolvedValue({ sequence: [], elapsedMs: [], values: {}, nextCursor: 'again' });
    await expect(
      loadSessionEnergy(controller, metadata, [{ startElapsedMs: 0, endElapsedMs: 1000 }], 10, null)
    ).rejects.toThrow('display limits');
  });
});
