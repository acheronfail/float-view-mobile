import type { LiveSample, LiveWidgetField, UnitPreference } from '$lib/backend/contract';
import { liveValue } from './live-values';
export const liveChartWindowMs = 30_000;
export const liveChartPointLimit = 1_500;
export interface LiveChartPoint {
  time: number;
  sample: LiveSample;
}
export const liveChartGroups: { title: string; fields: LiveWidgetField[] }[] = [
  { title: 'Speed', fields: ['speedKph'] },
  { title: 'Duty cycle', fields: ['dutyCycle'] },
  { title: 'Current', fields: ['motorCurrentA', 'batteryCurrentA'] },
  { title: 'Pack voltage', fields: ['packVoltageV'] },
  { title: 'Temperature', fields: ['controllerTempC', 'motorTempC'] },
  { title: 'Board angles', fields: ['pitchDeg', 'rollDeg', 'setpointDeg'] },
];
export function retainLivePoints(points: LiveChartPoint[], now: number): LiveChartPoint[] {
  return points.filter((point) => point.time >= now - liveChartWindowMs).slice(-liveChartPointLimit);
}
export function liveChartGeometry(
  points: LiveChartPoint[],
  fields: LiveWidgetField[],
  units: UnitPreference,
  now: number
) {
  const visible = retainLivePoints(points, now);
  const series = fields.map((field) =>
    visible.map((point) => ({ time: point.time, value: liveValue(field, point.sample, units).value }))
  );
  const values = series.flatMap((line) => line.flatMap((point) => (point.value === null ? [] : [point.value])));
  const minimum = Math.min(0, ...values);
  const maximum = Math.max(1, ...values);
  const padding = (maximum - minimum) * 0.08;
  const low = minimum < 0 ? minimum - padding : 0;
  const high = maximum + padding;
  return {
    low,
    high,
    paths: series.map((line) => {
      let previous: number | null = null;
      return line
        .map(({ time, value }) => {
          if (value === null) {
            previous = null;
            return '';
          }
          const command = previous === null || time - previous > 1_500 ? 'M' : 'L';
          previous = time;
          const x = ((time - (now - liveChartWindowMs)) / liveChartWindowMs) * 1000;
          const y = 100 - ((value - low) / (high - low)) * 100;
          return `${command}${x.toFixed(2)},${y.toFixed(2)}`;
        })
        .join(' ');
    }),
  };
}
