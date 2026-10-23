import { describe, it, expect } from 'vitest';
import { defaultDashboard, moveWidget, widgetPreviewSize, readingBarGeometry } from './custom-dashboard';
import { validCustomDashboard } from '$lib/backend/custom-dashboard';
describe('dashboard composition', () => {
  it('derives valid independent widget instances without empty live slots', () => {
    const dashboard = defaultDashboard(['empty', 'motorCurrentA', 'headlights', 'motorCurrentA', 'distanceM']);
    expect(validCustomDashboard(dashboard)).toBe(true);
    expect(dashboard.widgets.filter((w) => w.kind === 'reading' && w.field === 'motorCurrentA')).toHaveLength(2);
    expect(new Set(dashboard.widgets.map((w) => w.id)).size).toBe(dashboard.widgets.length);
  });
  it('previews the same bounded cell footprints and gaps used by the dashboard', () => {
    const grid = { columns: 4, cellSize: 85, rowSize: 56, gap: 6 };
    expect(widgetPreviewSize('compact', grid)).toEqual({ columns: 1, rows: 1, width: 85, height: 56 });
    expect(widgetPreviewSize('medium', grid)).toEqual({ columns: 2, rows: 2, width: 176, height: 118 });
    expect(widgetPreviewSize('large', grid)).toEqual({ columns: 4, rows: 2, width: 358, height: 118 });
    expect(widgetPreviewSize({ columns: 4, rows: 1 }, grid)).toEqual({ columns: 4, rows: 1, width: 358, height: 56 });
    expect(widgetPreviewSize({ columns: 1, rows: 4 }, { ...grid, columns: 2 })).toEqual({
      columns: 1,
      rows: 4,
      width: 85,
      height: 242,
    });
    expect(widgetPreviewSize('large', { ...grid, columns: 1 })).toEqual({ columns: 1, rows: 1, width: 85, height: 56 });
  });
  it('reorders by identity without mutating the accepted layout or changing widget configuration', () => {
    const dashboard = defaultDashboard(['empty', 'motorCurrentA', 'headlights', 'motorCurrentA', 'distanceM']);
    const first = dashboard.widgets[0];
    const moved = moveWidget(dashboard, first.id, 3);
    expect(moved.widgets[3]).toBe(first);
    expect(dashboard.widgets[0]).toBe(first);
    expect(moveWidget(dashboard, 'missing', 3)).toBe(dashboard);
  });
});

describe('reading bars', () => {
  it('extends from zero toward signed readings, with no fill at zero or unavailable', () => {
    const range = { min: -100, max: 100 };
    expect(readingBarGeometry(-50, range)).toEqual({ left: 0.25, width: 0.25, zero: 0.5 });
    expect(readingBarGeometry(50, range)).toEqual({ left: 0.5, width: 0.25, zero: 0.5 });
    for (const value of [0, null]) expect(readingBarGeometry(value, range)).toEqual({ left: 0.5, width: 0, zero: 0.5 });
    expect(readingBarGeometry(-200, range)).toEqual({ left: 0, width: 0.5, zero: 0.5 });
    expect(readingBarGeometry(200, range)).toEqual({ left: 0.5, width: 0.5, zero: 0.5 });
  });
  it('places zero truthfully for asymmetric ranges and preserves one-sided scales', () => {
    expect(readingBarGeometry(50, { min: -50, max: 150 })).toEqual({ left: 0.25, width: 0.25, zero: 0.25 });
    expect(readingBarGeometry(25, { min: 0, max: 100 })).toEqual({ left: 0, width: 0.25, zero: null });
    expect(readingBarGeometry(75, { min: 50, max: 100 })).toEqual({ left: 0, width: 0.5, zero: null });
    expect(readingBarGeometry(-55, { min: -100, max: -10 })).toEqual({ left: 0.5, width: 0.5, zero: null });
  });
});
