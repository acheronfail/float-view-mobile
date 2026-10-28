import {
  liveWidgetFields,
  MAX_DASHBOARD_BYTES,
  MAX_DASHBOARD_WIDGETS,
  type CustomDashboard,
  type DashboardReading,
  type DashboardRange,
  type DashboardSize,
  type DashboardDimensions,
  type UnitPreference,
} from './contract.ts';

/** Hard accepted bounds and useful initial domains, in metric display units. Keep Rust validation in sync. */
export function dashboardDomain(field: DashboardReading): { bounds: DashboardRange; initial: DashboardRange } | null {
  let bounds: [number, number], initial: [number, number];
  if (field.endsWith('TempC')) {
    bounds = [-100, 300];
    initial = [0, 100];
  } else if (field.endsWith('CurrentA')) {
    bounds = [-2000, 2000];
    initial = [-100, 100];
  } else if (field.endsWith('VoltageV') || field.endsWith('Adc1V') || field.endsWith('Adc2V')) {
    bounds = [0, 1000];
    initial = [0, field.includes('Cell') || field.includes('Adc') ? 5 : 100];
  } else if (
    [
      'dutyCycle',
      'driveLoad',
      'estimatedBatteryLevel',
      'batteryLevel',
      'bmsStateOfCharge',
      'bmsStateOfHealth',
      'bmsHumidityPercent',
    ].includes(field)
  ) {
    bounds = [0, 100];
    initial = [0, 100];
  } else if (field === 'speedKph' || field === 'maxSpeedKph') {
    bounds = [0, 500];
    initial = [0, 60];
  } else if (field.endsWith('Deg')) {
    bounds = [-360, 360];
    initial = [-30, 30];
  } else if (field === 'powerW') {
    bounds = [-100000, 100000];
    initial = [-5000, 5000];
  } else return null;
  return { bounds: { min: bounds[0], max: bounds[1] }, initial: { min: initial[0], max: initial[1] } };
}

export function dashboardRangeValue(
  field: DashboardReading,
  value: number,
  units: UnitPreference,
  inverse = false
): number {
  if (units === 'metric') return value;
  if (field.endsWith('TempC')) return inverse ? (value - 32) / 1.8 : value * 1.8 + 32;
  const factor =
    field === 'speedKph' || field === 'maxSpeedKph'
      ? 0.621371
      : field === 'distanceM' || field === 'odometerM'
        ? 1000 / 1609.344
        : 1;
  return inverse ? value / factor : value * factor;
}

const object = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);
const keys = (v: Record<string, unknown>, allowed: string[]) => Object.keys(v).every((k) => allowed.includes(k));
const namedDimensions = {
  compact: { columns: 1, rows: 1 },
  medium: { columns: 2, rows: 2 },
  large: { columns: 4, rows: 2 },
} as const;
export function dashboardDimensions(size: DashboardSize): DashboardDimensions {
  return typeof size === 'string' ? namedDimensions[size] : size;
}
export function validDashboardSize(size: unknown): size is DashboardSize {
  if (typeof size === 'string') return Object.hasOwn(namedDimensions, size);
  return (
    object(size) &&
    keys(size, ['columns', 'rows']) &&
    [size.columns, size.rows].every(
      (value) => typeof value === 'number' && Number.isInteger(value) && value >= 1 && value <= 4
    )
  );
}
export function validCustomDashboard(value: unknown): value is CustomDashboard {
  if (
    !object(value) ||
    !keys(value, ['version', 'widgets']) ||
    value.version !== 1 ||
    !Array.isArray(value.widgets) ||
    value.widgets.length > MAX_DASHBOARD_WIDGETS ||
    new TextEncoder().encode(JSON.stringify(value)).length > MAX_DASHBOARD_BYTES
  )
    return false;
  const ids = new Set<string>();
  return value.widgets.every((w) => {
    if (
      !object(w) ||
      typeof w.id !== 'string' ||
      !/^[A-Za-z0-9._-]{1,64}$/.test(w.id) ||
      ids.has(w.id) ||
      !validDashboardSize(w.size)
    )
      return false;
    ids.add(w.id);
    if (w.kind === 'visual')
      return (
        keys(w, ['id', 'size', 'kind', 'visual']) &&
        ['speedGauge', 'battery', 'temperatures', 'footpads'].includes(String(w.visual))
      );
    if (w.kind === 'action') return keys(w, ['id', 'size', 'kind', 'action']) && w.action === 'rideMarker';
    if (w.kind === 'control')
      return keys(w, ['id', 'size', 'kind', 'control']) && ['headlights', 'leds'].includes(String(w.control));
    if (
      w.kind !== 'reading' ||
      !keys(w, ['id', 'size', 'kind', 'field', 'presentation', 'range']) ||
      !(liveWidgetFields as readonly unknown[]).includes(w.field) ||
      ['empty', 'headlights', 'leds'].includes(String(w.field)) ||
      !['number', 'bar', 'gauge', 'trend'].includes(String(w.presentation))
    )
      return false;
    const domain = dashboardDomain(w.field as DashboardReading);
    if (['bar', 'gauge'].includes(String(w.presentation)) && !domain) return false;
    if ('range' in w) {
      const r = w.range;
      if (
        !domain ||
        !object(r) ||
        !keys(r, ['min', 'max']) ||
        typeof r.min !== 'number' ||
        typeof r.max !== 'number' ||
        !Number.isFinite(r.min) ||
        !Number.isFinite(r.max) ||
        r.min >= r.max ||
        r.min < domain.bounds.min ||
        r.max > domain.bounds.max
      )
        return false;
    }
    return true;
  });
}
