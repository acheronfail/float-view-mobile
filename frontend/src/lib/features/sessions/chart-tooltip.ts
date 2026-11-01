import type { SessionSeriesColumn } from '$lib/backend/session-data';
import { pointAtElapsed, type NumericDomain, type PlotPoint, type PlotPathOptions } from './history-geometry';
import type { SessionChartGroup } from './session-fields';

export interface ChartTooltipEntry {
  field: SessionChartGroup['fields'][number];
  point: PlotPoint;
}

export function formatChartElapsedTime(elapsedMs: number): string {
  const tenths = Math.round(elapsedMs / 100);
  const hours = Math.floor(tenths / 36_000);
  const minutes = Math.floor((tenths % 36_000) / 600);
  const seconds = (tenths % 600) / 10;
  const parts = [];
  if (hours) parts.push(`${hours}h`);
  if (minutes) parts.push(`${minutes}m`);
  if (seconds) parts.push(`${seconds}s`);
  return parts.join(' ') || '0s';
}

export function buildChartTooltipEntries(
  chartGroup: SessionChartGroup,
  columns: Readonly<Record<string, SessionSeriesColumn>> | null | undefined,
  hiddenFields: ReadonlySet<string>,
  elapsedMs: number,
  domain: NumericDomain,
  options: PlotPathOptions
): ChartTooltipEntry[] {
  return chartGroup.fields.flatMap((field) => {
    const column = columns?.[field.name];
    if (!column || hiddenFields.has(field.name)) return [];
    const point = pointAtElapsed(column, elapsedMs, domain, options);
    return point ? [{ field, point }] : [];
  });
}
