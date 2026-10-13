export type StableSessionFieldName =
  | 'packVoltageV'
  | 'dutyCycle'
  | 'controllerTempC'
  | 'motorTempC'
  | 'bmsCircuitryTempC'
  | 'bmsHighestCellTempC'
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
  readonly group?: keyof typeof sessionChartGroupMetadata;
  readonly color?: string;
  readonly axis?: SessionAxisPolicy;
  readonly format?: SessionFieldValueFormat;
  readonly precision?: number;
}

export interface SessionFieldDefinition {
  readonly displayName: string;
  readonly unit: string;
  readonly chartable?: boolean;
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
  torqueControl: { title: 'Torque control' },
  fieldWeakening: { title: 'Field weakening' },
  imu: { title: 'IMU' },
  footpads: { title: 'Footpads' },
  voltage: { title: 'Voltage' },
  setpoints: { title: 'Setpoints' },
  charge: { title: 'Charge' },
  energy: { title: 'Energy' },
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

/**
 * Note order of this defines the order of the charts in the session view!
 * Order of fields in the same group determines their order rendered on the chart.
 */
export const sessionFieldMetadata = {
  batteryLevel: {
    displayName: 'Battery level',
    unit: '%',
    chart: {
      axis: { kind: 'fixed', min: 0, max: 1 },
      color: 'var(--app-success)',
      format: 'percent',
      precision: 0,
    },
  },
  packVoltageV: {
    displayName: 'Pack voltage',
    unit: 'V',
    chart: {
      group: 'voltage',
      color: 'var(--app-accent)',
      precision: 2,
    },
  },
  motorCurrentA: {
    displayName: 'Motor current (draw/regen)',
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
  dutyCycle: {
    displayName: 'Duty cycle',
    unit: '%',
    chart: {
      axis: { kind: 'fixed', min: 0, max: 1 },
      color: 'var(--app-duty)',
      format: 'percent',
      precision: 0,
    },
  },
  speedKph: {
    displayName: 'Speed',
    unit: 'km/h',
    chart: {
      color: 'var(--app-success)',
      precision: 1,
    },
  },
  erpm: {
    displayName: 'ERPM',
    unit: 'electrical rpm',
    chart: {
      color: 'var(--app-accent)',
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
  bmsCircuitryTempC: {
    displayName: 'BMS temp',
    unit: '°C',
    chart: {
      group: 'temperature',
      color: '#a78bfa',
      precision: 1,
    },
  },
  bmsHighestCellTempC: {
    displayName: 'Cells temp',
    unit: '°C',
    chart: {
      group: 'temperature',
      color: '#2dd4bf',
      precision: 1,
    },
  },
  pitchDeg: {
    displayName: 'Pitch',
    unit: 'deg',
    chart: {
      group: 'imu',
      color: 'var(--app-success)',
      precision: 1,
    },
  },
  balancePitchDeg: {
    displayName: 'Balance pitch',
    unit: 'deg',

    chart: {
      group: 'imu',
      color: 'var(--app-danger)',
      precision: 1,
    },
  },
  rollDeg: {
    displayName: 'Roll',
    unit: 'deg',

    chart: {
      group: 'imu',
      color: 'var(--app-warning)',
      precision: 1,
    },
  },
  directionalMotorCurrentA: {
    displayName: 'Torque-producing current',
    unit: 'A',
    chart: {
      group: 'torqueControl',
      color: 'var(--app-accent)',
      precision: 2,
    },
  },
  filteredMotorCurrentA: {
    displayName: 'Smoothed torque current',
    unit: 'A',
    chart: {
      group: 'torqueControl',
      color: 'var(--app-success)',
      precision: 2,
    },
  },
  requestedBalanceCurrentA: {
    displayName: 'Balance current request',
    unit: 'A',
    chart: {
      group: 'torqueControl',
      color: 'var(--app-warning)',
      precision: 2,
    },
  },
  boosterCurrentA: {
    displayName: 'Booster contribution',
    unit: 'A',
    chartable: false,
  },
  fieldWeakeningCurrentA: {
    displayName: 'Field-weakening d-axis current',
    unit: 'A',
    chart: {
      group: 'fieldWeakening',
      color: 'var(--app-danger)',
      precision: 2,
    },
  },
  footpadAdc1V: {
    displayName: 'Footpad ADC 1',
    unit: 'V',
    chart: {
      group: 'footpads',
      color: 'var(--app-success)',
      precision: 2,
    },
  },
  footpadAdc2V: {
    displayName: 'Footpad ADC 2',
    unit: 'V',
    chart: {
      group: 'footpads',
      color: 'var(--app-warning)',
      precision: 2,
    },
  },
  remoteInput: {
    displayName: 'Remote input',
    unit: '%',
    chart: {
      axis: { kind: 'fixed', min: 0, max: 1 },
      color: 'var(--app-accent)',
      format: 'percent',
      precision: 0,
    },
  },
  setpointDeg: {
    displayName: 'Final setpoint',
    unit: 'deg',
    chart: {
      group: 'setpoints',
      color: 'var(--app-accent)',
      precision: 1,
    },
  },
  atrSetpointDeg: {
    displayName: 'ATR setpoint',
    unit: 'deg',
    chart: {
      group: 'setpoints',
      color: 'var(--app-success)',
      precision: 1,
    },
  },
  brakeTiltSetpointDeg: {
    displayName: 'Brake-tilt setpoint',
    unit: 'deg',
    chart: {
      group: 'setpoints',
      color: 'var(--app-warning)',
      precision: 1,
    },
  },
  torqueTiltSetpointDeg: {
    displayName: 'Torque-tilt setpoint',
    unit: 'deg',
    chart: {
      group: 'setpoints',
      color: 'var(--app-danger)',
      precision: 1,
    },
  },
  turnTiltSetpointDeg: {
    displayName: 'Turn-tilt setpoint',
    unit: 'deg',
    chart: {
      group: 'setpoints',
      color: 'var(--app-warning)',
      precision: 1,
    },
  },
  remoteSetpointDeg: {
    displayName: 'Remote setpoint',
    unit: 'deg',
    chart: {
      group: 'setpoints',
      color: 'var(--app-success)',
      precision: 1,
    },
  },
  atrAccelerationDifferenceErpmPerSecond: {
    displayName: 'ATR acceleration difference',
    unit: 'electrical rpm/s',
    chart: {
      color: 'var(--app-accent)',
      precision: 0,
    },
  },
  atrSpeedBoost: {
    displayName: 'ATR speed boost',
    unit: '%',
    chart: {
      axis: { kind: 'fixed', min: 0, max: 1 },
      color: 'var(--app-warning)',
      format: 'percent',
      precision: 0,
    },
  },
  distanceM: {
    displayName: 'Trip distance',
    unit: 'm',
    chartable: false,
  },
  odometerM: {
    displayName: 'Odometer',
    unit: 'm',
    chartable: false,
  },
  ampHoursDrawnAh: {
    displayName: 'Amp-hours drawn',
    unit: 'Ah',
    chart: {
      group: 'charge',
      color: 'var(--app-warning)',
      precision: 2,
    },
  },
  ampHoursChargedAh: {
    displayName: 'Amp-hours charged',
    unit: 'Ah',
    chart: {
      group: 'charge',
      color: 'var(--app-success)',
      precision: 2,
    },
  },
  wattHoursDrawnWh: {
    displayName: 'Watt-hours drawn',
    unit: 'Wh',
    chart: {
      group: 'energy',
      color: 'var(--app-warning)',
      precision: 1,
    },
  },
  wattHoursChargedWh: {
    displayName: 'Watt-hours charged',
    unit: 'Wh',
    chart: {
      group: 'energy',
      color: 'var(--app-success)',
      precision: 1,
    },
  },
  chargingCurrentA: {
    displayName: 'Charging current',
    unit: 'A',
    chart: {
      group: 'current',
      color: 'var(--app-success)',
      precision: 2,
    },
  },
  chargingVoltageV: {
    displayName: 'Charging voltage',
    unit: 'V',
    chart: {
      group: 'voltage',
      color: 'var(--app-success)',
      precision: 2,
    },
  },
} satisfies Record<StableSessionFieldName, SessionFieldDefinition>;

const sessionFieldMetadataOrder = new Map(Object.keys(sessionFieldMetadata).map((name, index) => [name, index]));

const DEFAULT_AXIS: SessionAxisPolicy = { kind: 'auto' };
const DEFAULT_COLOR = 'var(--app-warning)';
const DEFAULT_PRECISION = 2;
const MAX_PRECISION = 6;

function definitionFor(name: string): SessionFieldDefinition | null {
  return name in sessionFieldMetadata ? sessionFieldMetadata[name as StableSessionFieldName] : null;
}

export function sessionFieldIsChartable(name: string): boolean {
  return definitionFor(name)?.chartable !== false;
}

export function sortSessionFields<T extends { readonly name: string }>(fields: readonly T[]): T[] {
  return fields
    .map((field, index) => ({ field, index }))
    .sort((left, right) => {
      const leftOrder = sessionFieldMetadataOrder.get(left.field.name) ?? Infinity;
      const rightOrder = sessionFieldMetadataOrder.get(right.field.name) ?? Infinity;
      if (leftOrder !== rightOrder) return leftOrder - rightOrder;
      return left.index - right.index;
    })
    .map(({ field }) => field);
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
  for (const field of sortSessionFields(fields)) {
    if (!sessionFieldIsChartable(field.name)) continue;
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
