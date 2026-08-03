export type StableSessionFieldName =
  | 'packVoltageV'
  | 'dutyCycle'
  | 'controllerTempC'
  | 'motorTempC'
  | 'motorCurrentA'
  | 'batteryCurrentA'
  | 'erpm'
  | 'speedKph'
  | 'pitchDeg'
  | 'balancePitchDeg'
  | 'rollDeg'
  | 'directionalMotorCurrentA'
  | 'filteredMotorCurrentA'
  | 'requestedBalanceCurrentA'
  | 'boosterCurrentA'
  | 'fieldWeakeningCurrentA'
  | 'footpadAdc1V'
  | 'footpadAdc2V'
  | 'remoteInput'
  | 'setpointDeg'
  | 'atrSetpointDeg'
  | 'brakeTiltSetpointDeg'
  | 'torqueTiltSetpointDeg'
  | 'turnTiltSetpointDeg'
  | 'remoteSetpointDeg'
  | 'atrAccelerationDifferenceErpmPerSecond'
  | 'atrSpeedBoost'
  | 'distanceM'
  | 'odometerM'
  | 'ampHoursDrawnAh'
  | 'ampHoursChargedAh'
  | 'wattHoursDrawnWh'
  | 'wattHoursChargedWh'
  | 'batteryLevel'
  | 'chargingCurrentA'
  | 'chargingVoltageV';

export type SessionAxisPolicy =
  | { readonly kind: 'auto' }
  | { readonly kind: 'fixed'; readonly min: number; readonly max: number };

export type SessionFieldValueFormat = 'number' | 'percent';

export interface SessionChartGroupDefinition {
  readonly title: string;
}

export interface SessionFieldChartMetadata {
  readonly group?: string;
  readonly color?: string;
  readonly axis?: SessionAxisPolicy;
  readonly format?: SessionFieldValueFormat;
  readonly precision?: number;
}

export interface SessionFieldDefinition {
  readonly displayName: string;
  readonly unit: string;
  readonly chart?: SessionFieldChartMetadata;
}

export interface SessionChartField {
  readonly name: string;
  readonly displayName: string;
  readonly unit: string | null;
  readonly group: string;
  readonly color: string;
  readonly axis: SessionAxisPolicy;
  readonly format: SessionFieldValueFormat;
  readonly precision: number;
}

export const sessionChartGroupMetadata = {
  temperature: { title: 'Temperature' },
  current: { title: 'Current' },
} satisfies Record<string, SessionChartGroupDefinition>;

export interface SessionChartGroup {
  readonly id: string;
  readonly title: string;
  readonly unit: string | null;
  readonly fields: readonly SessionChartField[];
  readonly axis: SessionAxisPolicy;
  readonly format: SessionFieldValueFormat;
  readonly precision: number;
}

export const sessionFieldMetadata = {
  packVoltageV: {
    displayName: 'Pack voltage',
    unit: 'V',
  },
  dutyCycle: {
    displayName: 'Duty cycle',
    unit: '%',
    chart: {
      axis: { kind: 'fixed', min: 0, max: 1 },
      color: 'var(--app-warning)',
      format: 'percent',
      precision: 0,
    },
  },
  controllerTempC: {
    displayName: 'Controller temp',
    unit: '°C',
    chart: {
      group: 'temperature',
      color: 'var(--app-warning)',
      precision: 1,
    },
  },
  motorTempC: {
    displayName: 'Motor temp',
    unit: '°C',
    chart: {
      group: 'temperature',
      color: 'var(--app-danger)',
      precision: 1,
    },
  },
  motorCurrentA: {
    displayName: 'Motor current',
    unit: 'A',
    chart: {
      group: 'current',
      color: 'var(--app-warning)',
      precision: 2,
    },
  },
  batteryCurrentA: {
    displayName: 'Battery current',
    unit: 'A',
    chart: {
      group: 'current',
      color: 'var(--app-danger)',
      precision: 2,
    },
  },
  erpm: {
    displayName: 'ERPM',
    unit: 'electrical rpm',
    chart: {
      precision: 0,
    },
  },
  speedKph: {
    displayName: 'Speed',
    unit: 'km/h',
  },
  pitchDeg: {
    displayName: 'Pitch',
    unit: 'deg',
  },
  balancePitchDeg: {
    displayName: 'Balance pitch',
    unit: 'deg',
  },
  rollDeg: {
    displayName: 'Roll',
    unit: 'deg',
  },
  directionalMotorCurrentA: {
    displayName: 'Directional motor current',
    unit: 'A',
  },
  filteredMotorCurrentA: {
    displayName: 'Filtered motor current',
    unit: 'A',
  },
  requestedBalanceCurrentA: {
    displayName: 'Requested balance current',
    unit: 'A',
  },
  boosterCurrentA: {
    displayName: 'Booster current',
    unit: 'A',
  },
  fieldWeakeningCurrentA: {
    displayName: 'Field-weakening current',
    unit: 'A',
  },
  footpadAdc1V: {
    displayName: 'Footpad ADC 1',
    unit: 'V',
  },
  footpadAdc2V: {
    displayName: 'Footpad ADC 2',
    unit: 'V',
  },
  remoteInput: {
    displayName: 'Remote input',
    unit: 'ratio',
  },
  setpointDeg: {
    displayName: 'Final setpoint',
    unit: 'deg',
  },
  atrSetpointDeg: {
    displayName: 'ATR setpoint',
    unit: 'deg',
  },
  brakeTiltSetpointDeg: {
    displayName: 'Brake-tilt setpoint',
    unit: 'deg',
  },
  torqueTiltSetpointDeg: {
    displayName: 'Torque-tilt setpoint',
    unit: 'deg',
  },
  turnTiltSetpointDeg: {
    displayName: 'Turn-tilt setpoint',
    unit: 'deg',
  },
  remoteSetpointDeg: {
    displayName: 'Remote setpoint',
    unit: 'deg',
  },
  atrAccelerationDifferenceErpmPerSecond: {
    displayName: 'ATR acceleration difference',
    unit: 'electrical rpm/s',
  },
  atrSpeedBoost: {
    displayName: 'ATR speed boost',
    unit: 'ratio',
  },
  distanceM: {
    displayName: 'Trip distance',
    unit: 'm',
  },
  odometerM: {
    displayName: 'Odometer',
    unit: 'm',
  },
  ampHoursDrawnAh: {
    displayName: 'Amp-hours drawn',
    unit: 'Ah',
  },
  ampHoursChargedAh: {
    displayName: 'Amp-hours charged',
    unit: 'Ah',
  },
  wattHoursDrawnWh: {
    displayName: 'Watt-hours drawn',
    unit: 'Wh',
  },
  wattHoursChargedWh: {
    displayName: 'Watt-hours charged',
    unit: 'Wh',
  },
  batteryLevel: {
    displayName: 'Battery level',
    unit: 'ratio',
  },
  chargingCurrentA: {
    displayName: 'Charging current',
    unit: 'A',
  },
  chargingVoltageV: {
    displayName: 'Charging voltage',
    unit: 'V',
  },
} satisfies Record<StableSessionFieldName, SessionFieldDefinition>;

const DEFAULT_AXIS: SessionAxisPolicy = { kind: 'auto' };
const DEFAULT_COLOR = 'var(--app-accent)';
const DEFAULT_PRECISION = 2;
const MAX_PRECISION = 6;

function definitionFor(name: string): SessionFieldDefinition | null {
  return name in sessionFieldMetadata ? sessionFieldMetadata[name as StableSessionFieldName] : null;
}

function sameAxis(left: SessionAxisPolicy, right: SessionAxisPolicy): boolean {
  if (left.kind !== right.kind) return false;
  if (left.kind === 'auto' || right.kind === 'auto') return true;
  return left.min === right.min && left.max === right.max;
}

export function sessionChartField(name: string): SessionChartField {
  const definition = definitionFor(name);
  const chart = definition?.chart;
  const group = chart?.group ?? name;
  return {
    name,
    displayName: definition?.displayName ?? name,
    unit: definition?.unit ?? null,
    group,
    color: chart?.color ?? DEFAULT_COLOR,
    axis: chart?.axis ?? DEFAULT_AXIS,
    format: chart?.format ?? 'number',
    precision: Math.max(0, Math.min(MAX_PRECISION, Math.round(chart?.precision ?? DEFAULT_PRECISION))),
  };
}

export function sessionChartGroups(fields: readonly { name: string }[]): SessionChartGroup[] {
  const groups = new Map<string, SessionChartField[]>();
  for (const field of fields) {
    const chartField = sessionChartField(field.name);
    const group = groups.get(chartField.group);
    if (group) group.push(chartField);
    else groups.set(chartField.group, [chartField]);
  }

  return [...groups.entries()].map(([id, groupFields]) => {
    const first = groupFields[0]!;
    const axis = groupFields.every((field) => sameAxis(field.axis, first.axis)) ? first.axis : DEFAULT_AXIS;
    const format = groupFields.every((field) => field.format === first.format) ? first.format : 'number';
    const unit = groupFields.every((field) => field.unit === first.unit) ? first.unit : null;
    const precision = groupFields.every((field) => field.precision === first.precision)
      ? first.precision
      : Math.max(...groupFields.map((field) => field.precision));
    const groupDefinition = sessionChartGroupMetadata[id as keyof typeof sessionChartGroupMetadata];
    return {
      id,
      title: groupDefinition?.title ?? first.displayName,
      unit,
      fields: groupFields,
      axis,
      format,
      precision,
    };
  });
}

export function sessionFieldDisplayName(name: string): string {
  return definitionFor(name)?.displayName ?? name;
}

export function sessionFieldUnit(name: string): string | null {
  return definitionFor(name)?.unit ?? null;
}

export function sessionFieldValueFormat(name: string): SessionFieldValueFormat {
  return sessionChartField(name).format;
}

export function formatSessionFieldValue(name: string, value: number): string {
  if (!Number.isFinite(value)) return '';
  const precision = sessionChartField(name).precision;
  const scaled = sessionFieldValueFormat(name) === 'percent' ? value * 100 : value;
  const compact = scaled.toFixed(precision);
  return sessionFieldValueFormat(name) === 'percent' ? `${compact}%` : compact;
}
