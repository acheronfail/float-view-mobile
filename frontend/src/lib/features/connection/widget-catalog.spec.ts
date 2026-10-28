import { describe, expect, it } from 'vitest';
import { liveWidgetFields } from '$lib/backend/contract';
import { validCustomDashboard } from '$lib/backend/custom-dashboard';
import { pickerGroups, readingCategories, widgetCatalog } from './widget-catalog';
describe('widget discovery', () => {
  it('makes every reading discoverable exactly once with a valid Number/1×1 default', () => {
    const expected = liveWidgetFields.filter((field) => !['empty', 'headlights', 'leds'].includes(field));
    const actual = readingCategories.flatMap((category) => category.fields);
    expect([...actual].sort()).toEqual([...expected].sort());
    for (const entry of widgetCatalog) {
      expect(validCustomDashboard({ version: 1, widgets: [entry.widget] }), entry.name).toBe(true);
      if (entry.widget.kind === 'reading') {
        expect(entry.widget.presentation).toBe('number');
        expect(entry.widget.size).toEqual({ columns: 1, rows: 1 });
      }
    }
  });
  it('starts new combined gauges at 3×3', () => {
    expect(
      widgetCatalog.find((entry) => entry.widget.kind === 'visual' && entry.widget.visual === 'speedGauge')?.widget.size
    ).toEqual({ columns: 3, rows: 3 });
  });
  it('searches descriptive values and keeps battery sources and visual panels distinct', () => {
    const names = (query: string) =>
      pickerGroups(query, false).flatMap((group) => group.entries.map((entry) => entry.name));
    expect(names('battery level')).toEqual(
      expect.arrayContaining(['Estimated battery level', 'Controller-reported battery level', 'BMS battery level'])
    );
    expect(names('cell voltage spread')).toEqual(['Cell voltage spread']);
    expect(names('battery panel')).toEqual(['Battery panel']);
    expect(names('ride marker')).toEqual(['Save ride marker']);
    expect(names('button')).toEqual(['Save ride marker']);
    expect(names('nonexistent reading')).toEqual([]);
    expect(pickerGroups('', false).map((group) => group.name)).toEqual(['Values', 'Visual widgets', 'Controls']);
    expect(pickerGroups('', true).flatMap((group) => group.entries)).toHaveLength(widgetCatalog.length);
  });
});
