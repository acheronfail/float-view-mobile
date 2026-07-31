import type { SeriesPoint, SessionEvent } from '$lib/backend/contract';

export interface HistoricalField {
  key: string;
  label: string;
  unit: string;
}

export interface HistoricalGroup {
  title: string;
  fields: readonly HistoricalField[];
}

export const historicalTelemetryGroups: readonly HistoricalGroup[] = [
  {
    title: 'Motion',
    fields: [
      { key: 'speedKph', label: 'Speed', unit: 'km/h' },
      { key: 'erpm', label: 'ERPM', unit: 'ERPM' },
      { key: 'pitchDeg', label: 'Pitch', unit: '°' },
      { key: 'balancePitchDeg', label: 'Balance pitch', unit: '°' },
      { key: 'rollDeg', label: 'Roll', unit: '°' },
    ],
  },
  {
    title: 'Raw electrical load',
    fields: [
      { key: 'packVoltageV', label: 'Pack voltage', unit: 'V' },
      { key: 'dutyCycle', label: 'Duty ratio', unit: 'ratio' },
      { key: 'motorCurrentA', label: 'Actual motor current', unit: 'A' },
      { key: 'directionalMotorCurrentA', label: 'Directional motor current', unit: 'A' },
      { key: 'filteredMotorCurrentA', label: 'Filtered motor current', unit: 'A' },
      { key: 'batteryCurrentA', label: 'Battery current', unit: 'A' },
      { key: 'requestedBalanceCurrentA', label: 'Requested balance current', unit: 'A' },
      { key: 'boosterCurrentA', label: 'Booster current', unit: 'A' },
      { key: 'fieldWeakeningCurrentA', label: 'Field-weakening current', unit: 'A' },
    ],
  },
  {
    title: 'Temperatures and rider input',
    fields: [
      { key: 'controllerTempC', label: 'Controller temperature', unit: '°C' },
      { key: 'motorTempC', label: 'Motor temperature', unit: '°C' },
      { key: 'footpadAdc1V', label: 'Footpad ADC 1', unit: 'V' },
      { key: 'footpadAdc2V', label: 'Footpad ADC 2', unit: 'V' },
      { key: 'remoteInput', label: 'Remote input', unit: 'ratio' },
    ],
  },
  {
    title: 'Control contributions',
    fields: [
      { key: 'setpointDeg', label: 'Final setpoint', unit: '°' },
      { key: 'atrSetpointDeg', label: 'ATR setpoint', unit: '°' },
      { key: 'brakeTiltSetpointDeg', label: 'Brake-tilt setpoint', unit: '°' },
      { key: 'torqueTiltSetpointDeg', label: 'Torque-tilt setpoint', unit: '°' },
      { key: 'turnTiltSetpointDeg', label: 'Turn-tilt setpoint', unit: '°' },
      { key: 'remoteSetpointDeg', label: 'Remote setpoint', unit: '°' },
      {
        key: 'atrAccelerationDifferenceErpmPerSecond',
        label: 'ATR acceleration difference',
        unit: 'ERPM/s',
      },
      { key: 'atrSpeedBoost', label: 'ATR speed boost', unit: 'ratio' },
    ],
  },
  {
    title: 'Accumulated and charging',
    fields: [
      { key: 'distanceM', label: 'Trip distance', unit: 'm' },
      { key: 'odometerM', label: 'Odometer', unit: 'm' },
      { key: 'ampHoursDrawnAh', label: 'Amp-hours drawn', unit: 'Ah' },
      { key: 'ampHoursChargedAh', label: 'Amp-hours charged', unit: 'Ah' },
      { key: 'wattHoursDrawnWh', label: 'Watt-hours drawn', unit: 'Wh' },
      { key: 'wattHoursChargedWh', label: 'Watt-hours charged', unit: 'Wh' },
      { key: 'batteryLevel', label: 'Battery level', unit: 'ratio' },
      { key: 'chargingCurrentA', label: 'Charging current', unit: 'A' },
      { key: 'chargingVoltageV', label: 'Charging voltage', unit: 'V' },
    ],
  },
];

const stableHistoricalFields = new Set(
  historicalTelemetryGroups.flatMap((group) => group.fields.map((field) => field.key))
);

export function extensionHistoricalFields(points: readonly SeriesPoint[]): HistoricalField[] {
  return [
    ...new Set(
      points.flatMap((point) => Object.keys(point.values).filter((field) => !stableHistoricalFields.has(field)))
    ),
  ]
    .sort()
    .map((key) => ({ key, label: key, unit: 'source unit' }));
}

export function seriesValues(points: readonly SeriesPoint[], field: string) {
  return points
    .map((point) => ({ elapsedMs: point.elapsedMs, value: point.values[field] }))
    .filter(
      (point): point is { elapsedMs: number; value: number } =>
        typeof point.value === 'number' && Number.isFinite(point.value)
    );
}

export function seriesPolyline(points: readonly SeriesPoint[], field: string): string {
  const values = seriesValues(points, field);
  if (values.length === 0) return '';
  const first = values[0].elapsedMs;
  const elapsedRange = Math.max(1, values.at(-1)!.elapsedMs - first);
  const minimum = Math.min(...values.map((point) => point.value));
  const maximum = Math.max(...values.map((point) => point.value));
  const valueRange = Math.max(Number.EPSILON, maximum - minimum);
  return values
    .map(
      (point) =>
        `${((point.elapsedMs - first) / elapsedRange) * 320},${45 - ((point.value - minimum) / valueRange) * 40}`
    )
    .join(' ');
}

export function sessionEventText(event: SessionEvent): string {
  const kind = event.kind.replace(/([a-z])([A-Z])/g, '$1 $2').replace(/^./, (character) => character.toUpperCase());
  const value = event.value.replace(/([a-z])([A-Z])/g, '$1 $2').replace(/^./, (character) => character.toUpperCase());
  return `${kind}: ${value}`;
}
