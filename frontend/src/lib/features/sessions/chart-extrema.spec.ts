import { expect, it } from 'vitest';
import { chartExtrema, chartShowsMinimum } from './chart-extrema';
import { sessionChartGroups } from './session-fields';
import type { SessionSeriesColumn } from '$lib/backend/session-data';

const group = sessionChartGroups([{ name: 'controllerTempC' }, { name: 'motorTempC' }])[0]!;
const column = (values: (number | null)[]): SessionSeriesColumn => ({
  values,
  elapsedMs: [0, 10, 20],
  sequence: [1, 2, 3],
  generations: [0, 0, 0],
});
const columns = { controllerTempC: column([20, 30, 40]), motorTempC: column([10, 50, 60]) };
const range = { startElapsedMs: 0, endElapsedMs: 20 };
it('updates extrema for visible lines and handles all hidden', () => {
  expect(chartExtrema(group, columns, new Set(), range)).toEqual({ min: 10, max: 60 });
  expect(chartExtrema(group, columns, new Set(['motorTempC']), range)).toEqual({ min: 20, max: 40 });
  expect(chartExtrema(group, columns, new Set(['motorTempC', 'controllerTempC']), range)).toBeNull();
});
it('uses displayed time and map ranges and skips missing or non-finite values', () => {
  expect(chartExtrema(group, columns, new Set(), { startElapsedMs: 10, endElapsedMs: 10 })).toEqual({
    min: 30,
    max: 50,
  });
  expect(chartExtrema(group, columns, new Set(), range, [{ startElapsedMs: 0, endElapsedMs: 0 }])).toEqual({
    min: 10,
    max: 20,
  });
  expect(chartExtrema(group, columns, new Set(), range, [])).toBeNull();
  expect(chartExtrema(group, { motorTempC: column([null, NaN, Infinity]) }, new Set(), range)).toBeNull();
});
it('shows both temperature limits but only the useful Load maximum', () => {
  expect(chartShowsMinimum(group)).toBe(true);
  expect(chartShowsMinimum(sessionChartGroups([{ name: 'motorLoad' }])[0]!)).toBe(false);
});
