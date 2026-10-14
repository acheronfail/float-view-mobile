import type { SessionSeriesColumn } from '$lib/backend/session-data';
import type { SessionChartGroup } from './session-fields';
import type { Viewport } from './viewport';

export function chartShowsMinimum(group: SessionChartGroup): boolean {
  return !['load', 'speedKph', 'erpm', 'distanceM', 'odometerM', 'charge', 'energy'].includes(group.id);
}

/** Reduced series retain bucket extrema; never use padded axis bounds as readings. */
export function chartExtrema(
  group: SessionChartGroup,
  columns: Readonly<Record<string, SessionSeriesColumn>> | null | undefined,
  hidden: ReadonlySet<string>,
  viewport: Viewport,
  intervals?: readonly Viewport[]
): { min: number; max: number } | null {
  let min = Infinity;
  let max = -Infinity;
  for (const field of group.fields) {
    if (hidden.has(field.name)) continue;
    const column = columns?.[field.name];
    column?.values.forEach((value, index) => {
      const elapsed = column.elapsedMs[index];
      if (value == null || !Number.isFinite(value) || elapsed == null) return;
      if (elapsed < viewport.startElapsedMs || elapsed > viewport.endElapsedMs) return;
      if (intervals && !intervals.some((range) => elapsed >= range.startElapsedMs && elapsed <= range.endElapsedMs))
        return;
      min = Math.min(min, value);
      max = Math.max(max, value);
    });
  }
  return min === Infinity ? null : { min, max };
}
