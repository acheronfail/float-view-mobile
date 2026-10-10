import type { SessionSeriesColumn } from '$lib/backend/session-data';
import { pointAtElapsed, type NumericDomain, type PlotPoint } from './history-geometry';
import type { SessionChartGroup } from './session-fields';
import type { Viewport } from './viewport';

export interface ChartTooltipEntry {
  field: SessionChartGroup['fields'][number];
  point: PlotPoint;
}

export function buildChartTooltipEntries(
  chartGroup: SessionChartGroup,
  columns: Readonly<Record<string, SessionSeriesColumn>> | null | undefined,
  hiddenFields: ReadonlySet<string>,
  elapsedMs: number,
  domain: NumericDomain,
  options: { width: number; height: number; viewport: Viewport; intervals?: readonly Viewport[] }
): ChartTooltipEntry[] {
  return chartGroup.fields.flatMap((field) => {
    const column = columns?.[field.name];
    if (!column || hiddenFields.has(field.name)) return [];
    const point = pointAtElapsed(column, elapsedMs, domain, options);
    return point ? [{ field, point }] : [];
  });
}
