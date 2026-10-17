import { expect, it } from 'vitest';
import { chartDomain, chartExtrema, chartShowsMinimum } from './chart-extrema';
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

it('fits voltage to visible lines and restores the range when showing charging voltage', () => {
  const voltage = sessionChartGroups([{ name: 'packVoltageV' }, { name: 'chargingVoltageV' }])[0]!;
  const values = { packVoltageV: column([72, 74, 76]), chargingVoltageV: column([0, 0, 0]) };
  const hidden = new Set(['chargingVoltageV']);
  const original = chartDomain(voltage, values, new Set(), range)!;
  expect(original.min).toBeCloseTo(-3.8);
  expect(original.max).toBeCloseTo(79.8);
  expect(chartDomain(voltage, values, hidden, range)).toEqual({ min: 71.8, max: 76.2 });
  hidden.clear();
  expect(chartDomain(voltage, values, hidden, range)).toEqual(original);
});

it('fits automatic axes to the viewport and included map intervals', () => {
  expect(chartDomain(group, columns, new Set(['motorTempC']), { startElapsedMs: 0, endElapsedMs: 10 })).toEqual({
    min: 19.5,
    max: 30.5,
  });
  expect(chartDomain(group, columns, new Set(), range, [{ startElapsedMs: 0, endElapsedMs: 0 }])).toEqual({
    min: 9.5,
    max: 20.5,
  });
  expect(chartDomain(group, columns, new Set(), range, [])).toBeNull();
});

it('keeps percentage axes fixed while lines are hidden', () => {
  const load = sessionChartGroups([{ name: 'motorLoad' }, { name: 'batteryLoad' }])[0]!;
  const values = { motorLoad: column([0.2, 0.3, 0.4]), batteryLoad: column([0, 0, 0]) };
  expect(chartDomain(load, values, new Set(['batteryLoad']), range)).toEqual({ min: 0, max: 1 });
  expect(chartDomain(load, values, new Set(['batteryLoad', 'motorLoad']), range)).toBeNull();
});

it('handles hidden, missing, non-finite and constant readings without inventing extrema', () => {
  expect(chartDomain(group, columns, new Set(['controllerTempC', 'motorTempC']), range)).toBeNull();
  expect(chartDomain(group, undefined, new Set(), range)).toBeNull();
  expect(chartDomain(group, { motorTempC: column([null, NaN, Infinity]) }, new Set(), range)).toBeNull();
  expect(chartDomain(group, { motorTempC: column([20, 20, 20]) }, new Set(), range)).toEqual({ min: 19, max: 21 });
});
