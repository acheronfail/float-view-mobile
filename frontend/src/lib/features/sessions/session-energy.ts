import type { SessionMetadata } from '$lib/backend/contract';
import type { SessionReviewController } from '$lib/backend/session-controller';
import type { SessionSeriesColumn, SessionSeriesResult } from '$lib/backend/session-data';
import { intervalBuckets, MAX_MAP_INTERVALS } from './map-filter';
import type { Viewport } from './viewport';

export const ENERGY_COUNTERS = ['wattHoursDrawnWh', 'wattHoursChargedWh'] as const;
const MAX_SAMPLES = 1_000_000;
const MAX_PAGES = 1_000;
type Reading = { elapsedMs: number; sequence: number; value: number; generation: number };

/** Page exact retained counters before reducing: independently reduced counters
 * cannot be subtracted without losing net extrema. Memory stays O(buckets).
 * Always scan from the session start so zoom/map selection cannot change zero.
 */
export async function loadSessionEnergy(
  controller: SessionReviewController,
  metadata: SessionMetadata,
  intervals: readonly Viewport[],
  width: number,
  thresholdSeconds: number | null
): Promise<SessionSeriesResult> {
  if (metadata.storedSampleCount > MAX_SAMPLES) throw new Error('This session is too large to display session energy.');
  if (!intervals.length || intervals.length > MAX_MAP_INTERVALS) throw new Error('Invalid energy chart intervals.');
  const fields = ENERGY_COUNTERS.filter((name) => metadata.fields.some((field) => field.name === name));
  const names = fields.length === 2 ? [...fields, 'netEnergyWh'] : [...fields];
  const buckets = intervalBuckets(intervals, width);
  const selected = new Map(names.map((name) => [name, new Map<string, Reading[]>()]));
  const generations = new Map(names.map((name) => [name, 0]));
  const baseline = new Map<string, number>();
  const previous = new Map<string, number>();
  const invalid = new Set<string>();
  const thresholdMs =
    thresholdSeconds === null
      ? Math.max(3_000, (metadata.prune.targetResolutionSeconds ?? 0) * 3_000)
      : thresholdSeconds * 1_000;
  const endElapsedMs = intervals.at(-1)!.endElapsedMs;
  let lastElapsed: number | null = null;
  let lastSequence = -1;
  let lastInterval = -1;
  let cursor: string | null = null;
  let rows = 0;
  let pages = 0;
  do {
    if (++pages > MAX_PAGES) throw new Error('Session energy history exceeds display limits.');
    const page = await controller.loadSamples({
      kind: 'samples',
      mode: 'window',
      fields: [...fields],
      range: { startElapsedMs: 0, endElapsedMs: Math.max(0, Math.round(endElapsedMs)) },
      limit: 2_000,
      cursor,
    });
    rows += page.sequence.length;
    if (rows > MAX_SAMPLES || (page.nextCursor && !page.sequence.length))
      throw new Error('Session energy history exceeds display limits.');
    page.sequence.forEach((sequence, i) => {
      const elapsedMs = page.elapsedMs[i]!;
      if (lastElapsed !== null && (elapsedMs < lastElapsed || (elapsedMs === lastElapsed && sequence <= lastSequence)))
        throw new Error('Session energy history did not advance.');
      const interval = intervals.findIndex(
        (range) => elapsedMs >= range.startElapsedMs && elapsedMs <= range.endElapsedMs
      );
      const gap = lastElapsed !== null && elapsedMs - lastElapsed > thresholdMs;
      if (gap || interval !== lastInterval) names.forEach((name) => generations.set(name, generations.get(name)! + 1));
      lastElapsed = elapsedMs;
      lastSequence = sequence;
      lastInterval = interval;
      const raw = fields.map((name) => page.values[name]?.[i] ?? null);
      fields.forEach((name, index) => {
        const value = raw[index];
        if (value === null || !Number.isFinite(value) || value < 0) return;
        // A decrease loses an unknown amount of energy. Do not invent totals by
        // treating it as zero or splicing post-reset counters onto earlier ones.
        if (previous.has(name) && value < previous.get(name)!) invalid.add(name);
        previous.set(name, value);
      });
      if (!baseline.size && raw.every((value) => value !== null && Number.isFinite(value) && value >= 0))
        fields.forEach((name, index) => baseline.set(name, raw[index]!));
      const values = new Map<string, number | null>();
      fields.forEach((name, index) => {
        const value = raw[index];
        values.set(
          name,
          baseline.has(name) && !invalid.has(name) && value !== null && Number.isFinite(value) && value >= 0
            ? value - baseline.get(name)!
            : null
        );
      });
      if (fields.length === 2) {
        const drawn = values.get(ENERGY_COUNTERS[0]);
        const recovered = values.get(ENERGY_COUNTERS[1]);
        values.set('netEnergyWh', drawn != null && recovered != null ? drawn - recovered : null);
      }
      values.forEach((value, name) => {
        if (value === null) {
          generations.set(name, generations.get(name)! + 1);
          return;
        }
        if (interval < 0) return;
        const range = intervals[interval]!;
        const bucket = Math.min(
          buckets[interval]! - 1,
          Math.floor(
            ((elapsedMs - range.startElapsedMs) * buckets[interval]!) / (range.endElapsedMs - range.startElapsedMs + 1)
          )
        );
        const key = `${interval}:${bucket}`;
        const byBucket = selected.get(name)!;
        const current = [
          ...(byBucket.get(key) ?? []),
          { elapsedMs, sequence, value, generation: generations.get(name)! },
        ];
        const first = current[0]!,
          last = current.at(-1)!;
        const min = current.reduce((a, b) => (b.value < a.value ? b : a));
        const max = current.reduce((a, b) => (b.value > a.value ? b : a));
        byBucket.set(
          key,
          [...new Map([first, min, max, last].map((row) => [row.sequence, row])).values()].sort(
            (a, b) => a.elapsedMs - b.elapsedMs || a.sequence - b.sequence
          )
        );
      });
    });
    cursor = page.nextCursor;
  } while (cursor);
  const columns: Record<string, SessionSeriesColumn> = {};
  selected.forEach((byBucket, name) => {
    const readings = [...byBucket.values()].flat().sort((a, b) => a.elapsedMs - b.elapsedMs || a.sequence - b.sequence);
    columns[name] = {
      elapsedMs: readings.map((row) => row.elapsedMs),
      sequence: readings.map((row) => row.sequence),
      values: readings.map((row) => row.value),
      generations: readings.map((row) => row.generation),
    };
  });
  return {
    kind: 'series',
    sessionId: metadata.summary.sessionId,
    snapshot: metadata.snapshot,
    range: { startElapsedMs: intervals[0]!.startElapsedMs, endElapsedMs },
    gapPolicy: { thresholdSeconds },
    fields: names.map((name) => ({ name, source: 'stable' })),
    columns,
  };
}
