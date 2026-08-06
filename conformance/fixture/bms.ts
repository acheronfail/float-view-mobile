import type { FirmwareIdentity } from './boards.ts';

export type BmsTail = { kind: 'legacy' } | { kind: 'extended'; dataVersion: number; status: string };
export type BmsFailureMode =
  | 'none'
  | 'timeout'
  | 'excessive-cells'
  | 'excessive-temperatures'
  | 'invalid-balancing'
  | 'truncate-fixed'
  | 'truncate-cells'
  | 'truncate-balancing'
  | 'truncate-temperatures'
  | 'truncate-tail'
  | 'non-finite-auto-float'
  | 'reported-id-mismatch'
  | 'unterminated-status'
  | 'oversized-status'
  | 'invalid-status-utf8'
  | 'unexpected-remainder';

export const bmsFailureModes: readonly BmsFailureMode[] = [
  'none',
  'timeout',
  'excessive-cells',
  'excessive-temperatures',
  'invalid-balancing',
  'truncate-fixed',
  'truncate-cells',
  'truncate-balancing',
  'truncate-temperatures',
  'truncate-tail',
  'non-finite-auto-float',
  'reported-id-mismatch',
  'unterminated-status',
  'oversized-status',
  'invalid-status-utf8',
  'unexpected-remainder',
];

export interface BmsValues {
  totalVoltageV: number;
  chargeVoltageV: number;
  inputCurrentA: number;
  inputIcCurrentA: number;
  sessionAmpHoursAh: number;
  sessionWattHoursWh: number;
  cellVoltagesV: readonly number[];
  balancing: readonly boolean[];
  temperaturesC: readonly number[];
  icTemperatureC: number;
  humiditySensorTemperatureC: number;
  humidityPercent: number;
  highestCellTemperatureC: number;
  stateOfCharge: number;
  stateOfHealth: number;
  reportedCanId: number;
  lifetimeChargeAmpHoursAh: number;
  lifetimeChargeWattHoursWh: number;
  lifetimeDischargeAmpHoursAh: number;
  lifetimeDischargeWattHoursWh: number;
  pressurePa: number;
}

export interface BmsConfiguration {
  canId?: number;
  firmware: FirmwareIdentity;
  values: BmsValues;
  tail: BmsTail;
  pingVisible?: boolean;
  failureMode?: BmsFailureMode;
  valuesSource?: BmsValuesSource;
}

export interface BmsValuesSource {
  next(): BmsValues;
}

export interface InteractiveBmsSettings {
  seed: number;
  cellCount: number;
  temperatureCount: number;
  stateOfCharge: number;
  inputCurrentA: number;
  reportedCanId: number;
}

export const typicalBmsValues: BmsValues = {
  totalVoltageV: 50.4,
  chargeVoltageV: 54.6,
  inputCurrentA: -12.345,
  inputIcCurrentA: -12.3,
  sessionAmpHoursAh: 1.25,
  sessionWattHoursWh: 62.5,
  cellVoltagesV: [4.2, 4.19, 4.18],
  balancing: [true, false, true],
  temperaturesC: [25.5, 26.25],
  icTemperatureC: 27.5,
  humiditySensorTemperatureC: 24.75,
  humidityPercent: 52.5,
  highestCellTemperatureC: 29.25,
  stateOfCharge: 0.875,
  stateOfHealth: 0.95,
  reportedCanId: 42,
  lifetimeChargeAmpHoursAh: 64,
  lifetimeChargeWattHoursWh: 1024,
  lifetimeDischargeAmpHoursAh: 32,
  lifetimeDischargeWattHoursWh: 512,
  pressurePa: 101_320,
};

/** Deterministic, request-driven VBMS values for the interactive workbench. */
export class InteractiveBmsSource implements BmsValuesSource {
  private requestIndex = 0;
  private readonly settings: InteractiveBmsSettings;

  constructor(settings: InteractiveBmsSettings) {
    this.settings = settings;
    if (!Number.isInteger(settings.seed) || settings.seed < 0 || settings.seed > 0x7fffffff)
      throw new Error('VBMS seed is out of range');
    if (!Number.isInteger(settings.cellCount) || settings.cellCount < 1 || settings.cellCount > 50)
      throw new Error('VBMS cell count must be from 1 through 50');
    if (!Number.isInteger(settings.temperatureCount) || settings.temperatureCount < 0 || settings.temperatureCount > 50)
      throw new Error('VBMS temperature count must be from 0 through 50');
    if (!Number.isFinite(settings.stateOfCharge) || settings.stateOfCharge < 0 || settings.stateOfCharge > 1)
      throw new Error('VBMS state of charge must be from 0 through 1');
    if (!Number.isFinite(settings.inputCurrentA) || Math.abs(settings.inputCurrentA) > 200)
      throw new Error('VBMS input current is out of range');
    if (!Number.isInteger(settings.reportedCanId) || settings.reportedCanId < 0 || settings.reportedCanId > 253)
      throw new Error('VBMS reported CAN ID is out of range');
  }

  next(): BmsValues {
    const index = this.requestIndex++;
    const wave = Math.sin((this.settings.seed % 997) + index / 6);
    const baseCell = 3.2 + this.settings.stateOfCharge;
    const cellVoltagesV = Array.from({ length: this.settings.cellCount }, (_, cell) =>
      round(baseCell + Math.sin(this.settings.seed + cell * 1.7 + index / 10) * 0.006, 3)
    );
    const temperaturesC = Array.from({ length: this.settings.temperatureCount }, (_, sensor) =>
      round(24 + sensor * 0.35 + wave * 0.4, 2)
    );
    const totalVoltageV = round(
      cellVoltagesV.reduce((sum, value) => sum + value, 0),
      6
    );
    return {
      totalVoltageV,
      chargeVoltageV: round(this.settings.cellCount * 4.2, 6),
      inputCurrentA: round(this.settings.inputCurrentA + wave * 0.1, 6),
      inputIcCurrentA: round(this.settings.inputCurrentA + wave * 0.08, 6),
      sessionAmpHoursAh: round(1 + (index * Math.abs(this.settings.inputCurrentA)) / 3600, 3),
      sessionWattHoursWh: round(50 + (index * Math.abs(this.settings.inputCurrentA) * totalVoltageV) / 3600, 3),
      cellVoltagesV,
      balancing: cellVoltagesV.map((value) => value >= Math.max(...cellVoltagesV) - 0.002),
      temperaturesC,
      icTemperatureC: round(28 + wave * 0.5, 2),
      humiditySensorTemperatureC: round(23 + wave * 0.3, 2),
      humidityPercent: round(48 + wave * 2, 2),
      highestCellTemperatureC: round(Math.max(25, ...temperaturesC), 2),
      stateOfCharge: this.settings.stateOfCharge,
      stateOfHealth: 0.96,
      reportedCanId: this.settings.reportedCanId,
      lifetimeChargeAmpHoursAh: round(250 + index / 3600, 4),
      lifetimeChargeWattHoursWh: round(14_000 + index / 60, 3),
      lifetimeDischargeAmpHoursAh: round(230 + index / 3600, 4),
      lifetimeDischargeWattHoursWh: round(13_000 + index / 60, 3),
      pressurePa: Math.round(101_325 + wave * 20),
    };
  }
}

export function encodeBmsFirmwarePayload(hardwareName = 'VBMS', uuid = new Uint8Array(12)): Uint8Array {
  if (uuid.length !== 12) throw new Error('VBMS UUID must contain 12 bytes');
  const name = new TextEncoder().encode(hardwareName);
  if (name.length > 64) throw new Error('VBMS hardware name exceeds 64 bytes');
  return Uint8Array.from([0, 6, 0, ...name, 0, ...uuid, 0, 0, 1, 1]);
}

/** VESC Express 6.06 VBMS32 COMM_FW_VERSION identity at pinned commit a9f496c. */
export function encodeVbms32FirmwarePayload(efuseMac = new Uint8Array(6), hardwareCrc = new Uint8Array(4)): Uint8Array {
  if (efuseMac.length !== 6) throw new Error('VBMS32 eFuse MAC must contain 6 bytes');
  if (hardwareCrc.length !== 4) throw new Error('VBMS32 hardware CRC must contain 4 bytes');
  const hardwareName = new TextEncoder().encode('VBMS32');
  return Uint8Array.from([
    0,
    6,
    6,
    ...hardwareName,
    0,
    ...efuseMac,
    ...new Uint8Array(6),
    0, // pairing marker
    0, // test version
    2, // HW_TYPE_CUSTOM_MODULE
    1, // one custom configuration
    0, // phase filters
    0, // hardware QML
    0, // optional QML image flags
    0, // NRF flags
    0, // empty firmware name
    ...hardwareCrc,
  ]);
}

export function encodeBmsValuesPayload(values: BmsValues, tail: BmsTail = { kind: 'legacy' }): Uint8Array {
  const cells = values.cellVoltagesV;
  const temperatures = values.temperaturesC;
  if (cells.length > 50 || temperatures.length > 50 || values.balancing.length !== cells.length) {
    throw new Error('VBMS arrays exceed source bounds or do not align');
  }
  if (!values.balancing.every((value) => typeof value === 'boolean')) throw new Error('Invalid balancing value');
  if (values.reportedCanId < 0 || values.reportedCanId > 253) throw new Error('Invalid reported CAN ID');
  const bytes: number[] = [96];
  for (const [value, scale] of [
    [values.totalVoltageV, 1e6],
    [values.chargeVoltageV, 1e6],
    [values.inputCurrentA, 1e6],
    [values.inputIcCurrentA, 1e6],
    [values.sessionAmpHoursAh, 1e3],
    [values.sessionWattHoursWh, 1e3],
  ] as const)
    appendI32(bytes, value, scale);
  bytes.push(cells.length);
  cells.forEach((value) => appendI16(bytes, value, 1e3));
  values.balancing.forEach((value) => bytes.push(value ? 1 : 0));
  bytes.push(temperatures.length);
  temperatures.forEach((value) => appendI16(bytes, value, 1e2));
  for (const [value, scale] of [
    [values.icTemperatureC, 1e2],
    [values.humiditySensorTemperatureC, 1e2],
    [values.humidityPercent, 1e2],
    [values.highestCellTemperatureC, 1e2],
    [values.stateOfCharge, 1e3],
    [values.stateOfHealth, 1e3],
  ] as const)
    appendI16(bytes, value, scale);
  bytes.push(values.reportedCanId);
  [
    values.lifetimeChargeAmpHoursAh,
    values.lifetimeChargeWattHoursWh,
    values.lifetimeDischargeAmpHoursAh,
    values.lifetimeDischargeWattHoursWh,
  ].forEach((value) => bytes.push(...float32Auto(value)));
  appendI16(bytes, values.pressurePa, 1e-1);
  if (tail.kind === 'extended') {
    const status = new TextEncoder().encode(tail.status);
    if (tail.dataVersion < 0 || tail.dataVersion > 255 || status.length > 40) {
      throw new Error('Invalid VBMS extended tail');
    }
    bytes.push(tail.dataVersion, ...status, 0);
  }
  return Uint8Array.from(bytes);
}

function appendI16(bytes: number[], value: number, scale: number) {
  const raw = Math.trunc(value * scale);
  if (!Number.isFinite(value) || raw < -32768 || raw > 32767) throw new Error('VBMS int16 overflow');
  bytes.push((raw >> 8) & 0xff, raw & 0xff);
}

function appendI32(bytes: number[], value: number, scale: number) {
  const raw = Math.trunc(value * scale);
  if (!Number.isFinite(value) || raw < -2147483648 || raw > 2147483647) throw new Error('VBMS int32 overflow');
  bytes.push((raw >> 24) & 0xff, (raw >> 16) & 0xff, (raw >> 8) & 0xff, raw & 0xff);
}

function float32Auto(value: number): number[] {
  if (!Number.isFinite(value)) throw new Error('VBMS auto float must be finite');
  if (value === 0) return [0, 0, 0, 0];
  const exponent = Math.floor(Math.log2(Math.abs(value))) + 1;
  const significand = Math.abs(value) / 2 ** exponent;
  const significandBits = Math.trunc((significand - 0.5) * 2 * 8388608) & 0x7fffff;
  const raw = ((exponent + 126) << 23) | significandBits | (value < 0 ? 0x80000000 : 0);
  return [(raw >>> 24) & 0xff, (raw >>> 16) & 0xff, (raw >>> 8) & 0xff, raw & 0xff];
}

function round(value: number, digits: number) {
  const scale = 10 ** digits;
  return Math.round(value * scale) / scale;
}
