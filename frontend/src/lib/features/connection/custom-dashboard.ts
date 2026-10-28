import type { CustomDashboard, DashboardWidget, DashboardSize, LiveWidgetSlots } from '$lib/backend/contract';
import { dashboardDimensions } from '$lib/backend/custom-dashboard';
import { liveValueCatalog } from './live-values';

export type DashboardGridGeometry = { columns: number; cellSize: number; rowSize: number; gap: number };

export function widgetPreviewSize(size: DashboardSize, grid: DashboardGridGeometry) {
  const intent = dashboardDimensions(size);
  const columns = Math.min(intent.columns, grid.columns);
  const rows = typeof size === 'string' ? Math.min(intent.rows, grid.columns) : intent.rows;
  return {
    columns,
    rows,
    width: columns * grid.cellSize + (columns - 1) * grid.gap,
    height: rows * grid.rowSize + (rows - 1) * grid.gap,
  };
}

export function readingBarGeometry(value: number | null, range: { min: number; max: number }) {
  const position = (reading: number) => Math.max(0, Math.min(1, (reading - range.min) / (range.max - range.min)));
  const origin = range.min < 0 ? position(0) : 0;
  const end = value == null ? origin : position(value);
  return {
    left: Math.min(origin, end),
    width: Math.abs(end - origin),
    zero: range.min < 0 && range.max > 0 ? origin : null,
  };
}

export function widgetTitle(widget: DashboardWidget): string {
  if (widget.kind === 'action') return 'Save ride marker';
  if (widget.kind === 'reading') return liveValueCatalog[widget.field].name;
  if (widget.kind === 'control') return liveValueCatalog[widget.control].name;
  return { speedGauge: 'Speed and load', battery: 'Battery', temperatures: 'Temperatures', footpads: 'Footpads' }[
    widget.visual
  ];
}
export function defaultDashboard(slots: LiveWidgetSlots): CustomDashboard {
  const widgets: DashboardWidget[] = slots.flatMap((field, i): DashboardWidget[] =>
    field === 'empty'
      ? []
      : [
          field === 'headlights' || field === 'leds'
            ? { id: `slot-${i}`, size: 'compact', kind: 'control', control: field }
            : { id: `slot-${i}`, size: 'compact', kind: 'reading', field, presentation: 'number' },
        ]
  );
  widgets.push(
    { id: 'speed', size: 'large', kind: 'visual', visual: 'speedGauge' },
    { id: 'battery', size: 'medium', kind: 'visual', visual: 'battery' },
    { id: 'temperatures', size: 'medium', kind: 'visual', visual: 'temperatures' },
    { id: 'footpads', size: 'compact', kind: 'visual', visual: 'footpads' }
  );
  return { version: 1, widgets };
}
export function moveWidget(dashboard: CustomDashboard, id: string, to: number): CustomDashboard {
  const widgets = [...dashboard.widgets];
  const from = widgets.findIndex((w) => w.id === id);
  if (from < 0) return dashboard;
  const [widget] = widgets.splice(from, 1);
  widgets.splice(Math.max(0, Math.min(widgets.length, to)), 0, widget);
  return { ...dashboard, widgets };
}
