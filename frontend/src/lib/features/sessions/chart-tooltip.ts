import type { SessionSeriesColumn } from '$lib/backend/session-data';
import { pointAtElapsed, type NumericDomain, type PlotPoint, type PlotPathOptions } from './history-geometry';
import type { SessionChartGroup } from './session-fields';

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
  options: PlotPathOptions
): ChartTooltipEntry[] {
  return chartGroup.fields.flatMap((field) => {
    const column = columns?.[field.name];
    if (!column || hiddenFields.has(field.name)) return [];
    const point = pointAtElapsed(column, elapsedMs, domain, options);
    return point ? [{ field, point }] : [];
  });
}
