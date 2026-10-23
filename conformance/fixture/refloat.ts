import type {
  AlertReason,
  FirmwareFault,
  FootpadState,
  PackageMode,
  PackageState,
  SetpointAdjustment,
  StopCondition,
  TelemetrySample,
} from './telemetry.ts';

export type RefloatVersion =
  '1.0.0' | '1.0.1' | '1.1.0' | '1.1.1' | '1.1.2' | '1.2.0' | '1.2.1' | '1.2.2' | '1.2.3' | '1.3.0';
type DynamicRefloatVersion = Exclude<RefloatVersion, '1.0.0' | '1.0.1'>;
export type RefloatRequestKind = 'info' | 'layout' | 'telemetry' | 'compact';

export const MAX_REFLOAT_FIELDS = 64;
export const MAX_REFLOAT_FIELD_BYTES = 64;
export const MAX_REFLOAT_FIELD_NAME_BYTES = 4_096;

export interface RefloatResponse {
  kind: RefloatRequestKind;
  payload: Uint8Array;
}

export interface RefloatCodec {
  version: RefloatVersion;
  provenance: string;
  infoPayload: Uint8Array;
  layoutPayload?: Uint8Array;
  respond(payload: Uint8Array, sample: TelemetrySample): RefloatResponse | undefined;
}

/** Source order from src/rt_data.h in the supported Refloat tags. */
export const realtimeFields = [
  'motor.speed',
  'motor.erpm',
  'motor.current',
  'motor.dir_current',
  'motor.filt_current',
  'motor.duty_cycle',
  'motor.batt_voltage',
  'motor.batt_current',
  'motor.mosfet_temp',
  'motor.motor_temp',
  'imu.pitch',
  'imu.balance_pitch',
  'imu.roll',
  'footpad.adc1',
  'footpad.adc2',
  'remote.input',
] as const;

export const runtimeRealtimeFields = [
  'setpoint',
  'atr.setpoint',
  'brake_tilt.setpoint',
  'torque_tilt.setpoint',
  'turn_tilt.setpoint',
  'remote.setpoint',
  'balance_current',
  'atr.accel_diff',
  'atr.speed_boost',
  'booster.current',
] as const;

const provenance =
  'Immutable read-only wire profiles audited from stable https://github.com/lukash/refloat tags v1.0.0 through v1.3.0';

export const refloatCodecs: Readonly<Record<RefloatVersion, RefloatCodec>> = {
  '1.0.0': legacyCodec('1.0.0'),
  '1.0.1': legacyCodec('1.0.1'),
  '1.1.0': dynamicCodec('1.1.0', false),
  '1.1.1': dynamicCodec('1.1.1', false),
  '1.1.2': dynamicCodec('1.1.2', false),
  '1.2.0': dynamicCodec('1.2.0', true),
  '1.2.1': dynamicCodec('1.2.1', true),
  '1.2.2': dynamicCodec('1.2.2', true),
  '1.2.3': dynamicCodec('1.2.3', true),
  '1.3.0': selectableCodec(),
};

export function encodeRefloatTelemetry(version: RefloatVersion, sample: TelemetrySample): Uint8Array {
  if (version === '1.3.0') {
    return encodeSelectableTelemetry(Uint8Array.from([36, 101, 33, 1, 0x7f, 0xff, 0xbf, 0xc3, 0, 0, 0, 0]), sample);
  }
  return version.startsWith('1.0.')
    ? encodeLegacyCompact(sample, 4)
    : encodeDynamicTelemetry(sample, version.startsWith('1.2.'));
}

export function encodeRefloatLayout(
  fields: readonly string[] = realtimeFields,
  runtimeFields: readonly string[] = runtimeRealtimeFields
): Uint8Array {
  validateLayout(fields, runtimeFields);
  const bytes: number[] = [36, 101, 32];
  appendFieldList(bytes, fields);
  appendFieldList(bytes, runtimeFields);
  return Uint8Array.from(bytes);
}

export function encodeLegacyRealtime(sample: TelemetrySample): Uint8Array {
  // Upstream's 72-byte app-data body is wrapped by VESC command 36.
  const payload = new Uint8Array(73);
  payload.set([36, 101, 1]);
  let offset = 3;
  offset = appendFloat32Auto(payload, offset, sample.requestedBalanceCurrentA);
  offset = appendFloat32Auto(payload, offset, sample.balancePitchDeg);
  offset = appendFloat32Auto(payload, offset, sample.rollDeg);
  payload[offset++] = legacyState(sample) | (legacySetpointAdjustment(sample.setpointAdjustment) << 4);
  payload[offset++] =
    legacyFootpad(sample.footpadState) |
    (sample.packageMode === 'handTest' ? 0x08 : 0) |
    (alertReasonCode(sample.alertReason) << 4);
  offset = appendFloat32Auto(payload, offset, sample.footpadAdc1V);
  offset = appendFloat32Auto(payload, offset, sample.footpadAdc2V);
  for (const value of [
    sample.setpointDeg,
    sample.atrSetpointDeg,
    sample.brakeTiltSetpointDeg,
    sample.torqueTiltSetpointDeg,
    sample.turnTiltSetpointDeg,
    sample.remoteSetpointDeg,
    sample.pitchDeg,
    sample.filteredMotorCurrentA,
    sample.atrAccelerationDifferenceErpmPerSecond,
  ]) {
    offset = appendFloat32Auto(payload, offset, value);
  }
  if (sample.charging) {
    offset = appendFloat32Auto(payload, offset, sample.chargingCurrentA);
    offset = appendFloat32Auto(payload, offset, sample.chargingVoltageV);
  } else {
    offset = appendFloat32Auto(payload, offset, sample.boosterCurrentA);
    offset = appendFloat32Auto(payload, offset, sample.directionalMotorCurrentA);
  }
  appendFloat32Auto(payload, offset, sample.remoteInput);
  return payload;
}

export function encodeLegacyCompact(sample: TelemetrySample, detailLevel: number): Uint8Array {
  if (!Number.isInteger(detailLevel) || detailLevel < 1 || detailLevel > 4) {
    throw new Error('Refloat compact detail level must be 1 through 4');
  }
  if (sample.firmwareFault !== null) {
    return Uint8Array.of(36, 101, 10, 69, firmwareFaultCode(sample.firmwareFault));
  }
  const length = detailLevel >= 4 ? 59 : detailLevel >= 3 ? 55 : detailLevel >= 2 ? 42 : 35;
  const payload = new Uint8Array(length);
  payload.set([36, 101, 10, detailLevel]);
  i16(payload, 4, sample.requestedBalanceCurrentA * 10);
  i16(payload, 6, sample.balancePitchDeg * 10);
  i16(payload, 8, sample.rollDeg * 10);
  payload[10] = legacyState(sample) | (legacySetpointAdjustment(sample.setpointAdjustment) << 4);
  payload[11] =
    legacyFootpad(sample.footpadState) |
    (sample.packageMode === 'handTest' ? 0x08 : 0) |
    (alertReasonCode(sample.alertReason) << 4);
  payload[12] = byte(sample.footpadAdc1V * 50);
  payload[13] = byte(sample.footpadAdc2V * 50);
  const setpoints = [
    sample.setpointDeg,
    sample.atrSetpointDeg,
    sample.brakeTiltSetpointDeg,
    sample.torqueTiltSetpointDeg,
    sample.turnTiltSetpointDeg,
    sample.remoteSetpointDeg,
  ];
  setpoints.forEach((value, index) => {
    payload[14 + index] = byte(value * 5 + 128);
  });
  i16(payload, 20, sample.pitchDeg * 10);
  payload[22] = byte(sample.boosterCurrentA + 128);
  i16(payload, 23, sample.packVoltageV * 10);
  i16(payload, 25, sample.erpm);
  i16(payload, 27, (sample.speedKph / 3.6) * 10);
  i16(payload, 29, sample.motorCurrentA * 10);
  i16(payload, 31, sample.batteryCurrentA * 10);
  payload[33] = byte(sample.dutyCycle * 100 + 128);
  payload[34] = byte(Math.abs(sample.fieldWeakeningCurrentA) * 3);
  if (detailLevel >= 2) {
    appendFloat32Auto(payload, 35, sample.distanceM);
    payload[39] = byte(Math.max(0, sample.controllerTempC * 2));
    payload[40] = byte(Math.max(0, sample.motorTempC * 2));
    payload[41] = 0;
  }
  if (detailLevel >= 3) {
    u32(payload, 42, sample.odometerM);
    i16(payload, 46, sample.ampHoursDrawnAh * 10);
    i16(payload, 48, sample.ampHoursChargedAh * 10);
    i16(payload, 50, sample.wattHoursDrawnWh);
    i16(payload, 52, sample.wattHoursChargedWh);
    payload[54] = byte(clamp(sample.batteryLevel, 0, 1.25) * 200);
  }
  if (detailLevel >= 4) {
    i16(payload, 55, sample.chargingCurrentA * 10);
    i16(payload, 57, sample.chargingVoltageV * 10);
  }
  return payload;
}

export function encodeDynamicTelemetry(sample: TelemetrySample, alertTail: boolean): Uint8Array {
  const runtime = sample.packageState === 'running';
  let mask = runtime ? 0x01 : 0;
  if (sample.charging) mask |= 0x02;
  if (alertTail) mask |= 0x04;
  const values = [
    sample.speedKph,
    sample.erpm,
    sample.motorCurrentA,
    sample.directionalMotorCurrentA,
    sample.filteredMotorCurrentA,
    sample.dutyCycle,
    sample.packVoltageV,
    sample.batteryCurrentA,
    sample.controllerTempC,
    sample.motorTempC,
    sample.pitchDeg,
    sample.balancePitchDeg,
    sample.rollDeg,
    sample.footpadAdc1V,
    sample.footpadAdc2V,
    sample.remoteInput,
    ...(runtime
      ? [
          sample.setpointDeg,
          sample.atrSetpointDeg,
          sample.brakeTiltSetpointDeg,
          sample.torqueTiltSetpointDeg,
          sample.turnTiltSetpointDeg,
          sample.remoteSetpointDeg,
          sample.requestedBalanceCurrentA,
          sample.atrAccelerationDifferenceErpmPerSecond,
          sample.atrSpeedBoost,
          sample.boosterCurrentA,
        ]
      : []),
    ...(sample.charging ? [sample.chargingCurrentA, sample.chargingVoltageV] : []),
  ];
  const payload = new Uint8Array(13 + values.length * 2 + (alertTail ? 9 : 0));
  payload.set([36, 101, 31, mask, 0, 0, 0, 0, 0]);
  payload[9] = packageStateCode(sample.packageState) | (packageModeCode(sample.packageMode) << 4);
  payload[10] =
    (footpadStateCode(sample.footpadState) << 6) |
    (sample.charging ? 0x20 : 0) |
    (sample.darkride ? 0x02 : 0) |
    (sample.wheelslip ? 0x01 : 0);
  payload[11] = stopConditionCode(sample.stopCondition) | (setpointAdjustmentCode(sample.setpointAdjustment) << 4);
  payload[12] = alertReasonCode(sample.alertReason);
  let offset = 13;
  for (const value of values) {
    const bits = float16(value);
    payload[offset++] = bits >> 8;
    payload[offset++] = bits & 0xff;
  }
  if (alertTail) {
    const activeAlertMask = sample.firmwareFault === null ? 0 : 1;
    u32(payload, offset, activeAlertMask);
    u32(payload, offset + 4, 0);
    payload[offset + 8] = sample.firmwareFault === null ? 0 : firmwareFaultCode(sample.firmwareFault);
  }
  return payload;
}

function legacyCodec(version: '1.0.0' | '1.0.1'): RefloatCodec {
  const infoPayload = Uint8Array.of(36, 101, 0, 10);
  return {
    version,
    provenance,
    infoPayload,
    respond(payload, sample) {
      const packet = packagePacket(payload);
      if (packet === 0) return { kind: 'info', payload: infoPayload };
      if (packet === 1) return { kind: 'telemetry', payload: encodeLegacyRealtime(sample) };
      if (packet === 10) {
        return { kind: 'compact', payload: encodeLegacyCompact(sample, payload[3] ?? 4) };
      }
      return undefined;
    },
  };
}

function dynamicCodec(version: DynamicRefloatVersion, alertTail: boolean): RefloatCodec {
  const infoPayload = encodeInfoV2(version);
  const layoutPayload = encodeRefloatLayout();
  return {
    version,
    provenance,
    infoPayload,
    layoutPayload,
    respond(payload, sample) {
      const packet = packagePacket(payload);
      if (packet === 0) return { kind: 'info', payload: infoPayload };
      if (packet === 32) return { kind: 'layout', payload: layoutPayload };
      if (packet === 31) {
        return { kind: 'telemetry', payload: encodeDynamicTelemetry(sample, alertTail) };
      }
      if (packet === 10) {
        return { kind: 'compact', payload: encodeLegacyCompact(sample, payload[3] ?? 4) };
      }
      return undefined;
    },
  };
}

function selectableCodec(): RefloatCodec {
  const version = '1.3.0' as const;
  const infoPayload = encodeInfoV2(version);
  infoPayload[59] = 0x04;
  return {
    version,
    provenance,
    infoPayload,
    respond(payload, sample) {
      const packet = packagePacket(payload);
      if (packet === 0) return { kind: 'info', payload: infoPayload };
      if (packet === 33 && payload.length === 12 && payload[3] === 1) {
        return { kind: 'telemetry', payload: encodeSelectableTelemetry(payload, sample) };
      }
      if (packet === 10 && payload.length === 4 && payload[3] === 0) {
        const response = encodeLegacyCompact(sample, 1);
        if (sample.firmwareFault === null) response[3] = 0;
        return { kind: 'compact', payload: response };
      }
      return undefined;
    },
  };
}

export function encodeSelectableTelemetry(request: Uint8Array, sample: TelemetrySample): Uint8Array {
  const view = new DataView(request.buffer, request.byteOffset, request.byteLength);
  const mask1 = view.getUint32(4, false);
  const mask2 = view.getUint32(8, false);
  const bytes: number[] = [36, 101, 33, 1];
  pushU32(bytes, mask1);
  pushU32(bytes, mask2);
  pushU32(bytes, 0);
  if (mask1 & (1 << 0)) bytes.push(0);
  if (mask1 & (1 << 1)) {
    let state = packageModeCode(sample.packageMode) << 28;
    state |= packageStateCode(sample.packageState) << 24;
    state |= footpadStateCode(sample.footpadState) << 22;
    if (sample.charging) state |= 1 << 21;
    if (sample.darkride) state |= 1 << 17;
    if (sample.wheelslip) state |= 1 << 16;
    state |= setpointAdjustmentCode(sample.setpointAdjustment) << 12;
    state |= stopConditionCode(sample.stopCondition) << 8;
    state |= alertReasonCode(sample.alertReason);
    pushU32(bytes, state >>> 0);
  }
  const values1: ReadonlyArray<readonly [number, number]> = [
    [6, sample.speedKph],
    [7, sample.erpm],
    [8, sample.motorCurrentA],
    [9, sample.directionalMotorCurrentA],
    [10, sample.filteredMotorCurrentA],
    [11, sample.dutyCycle],
    [12, sample.packVoltageV],
    [13, sample.batteryCurrentA],
    [14, sample.batteryLevel],
    [15, sample.controllerTempC],
    [16, sample.motorTempC],
    [17, sample.pitchDeg],
    [18, sample.balancePitchDeg],
    [19, sample.rollDeg],
    [20, sample.footpadAdc1V],
    [21, sample.footpadAdc2V],
    [22, sample.remoteInput],
    [23, sample.setpointDeg],
    [24, sample.atrSetpointDeg],
    [25, sample.brakeTiltSetpointDeg],
    [26, sample.torqueTiltSetpointDeg],
    [27, sample.turnTiltSetpointDeg],
    [28, sample.remoteSetpointDeg],
    [29, sample.requestedBalanceCurrentA],
    [30, 1000],
  ];
  for (const [bit, value] of values1) if (mask1 & (1 << bit)) pushF32(bytes, value);
  if (mask2 & (1 << 0)) pushU32(bytes, sample.odometerM);
  const values2: ReadonlyArray<readonly [number, number]> = [
    [1, sample.distanceM],
    [2, sample.chargingVoltageV],
    [3, sample.chargingCurrentA],
    [4, sample.ampHoursDrawnAh],
    [5, sample.ampHoursChargedAh],
    [6, sample.wattHoursDrawnWh],
    [7, sample.wattHoursChargedWh],
    [8, sample.fieldWeakeningCurrentA],
  ];
  for (const [bit, value] of values2) if (mask2 & (1 << bit)) pushF32(bytes, value);
  if (mask2 & (1 << 9)) pushF64(bytes, -33.8688);
  if (mask2 & (1 << 10)) pushF64(bytes, 151.2093);
  if (mask2 & (1 << 11)) pushF32(bytes, 10);
  if (mask2 & (1 << 12)) pushF32(bytes, sample.speedKph);
  if (mask2 & (1 << 13)) pushF32(bytes, 0.8);
  if (mask2 & (1 << 14)) pushU32(bytes, 1);
  return Uint8Array.from(bytes);
}

function packagePacket(payload: Uint8Array): number | undefined {
  if (payload[0] !== 36 || payload[1] !== 101) return undefined;
  return payload[2];
}

function encodeInfoV2(version: DynamicRefloatVersion): Uint8Array {
  const payload = new Uint8Array(61);
  payload.set([36, 101, 0, 2]);
  payload.set(new TextEncoder().encode('Refloat').slice(0, 20), 5);
  const [major, minor, patch] = version.split('.').map(Number) as [number, number, number];
  payload.set([major, minor, patch], 25);
  // Upstream stable builds pass the undelimited patch through `cut -d- -f2-`.
  payload.set(new TextEncoder().encode(String(patch)), 28);
  return payload;
}

function validateLayout(fields: readonly string[], runtimeFields: readonly string[]): void {
  const all = [...fields, ...runtimeFields];
  if (all.length > MAX_REFLOAT_FIELDS) throw new Error('Refloat field layout exceeds 64 fields');
  const encoded = all.map((field) => new TextEncoder().encode(field));
  if (encoded.some((field) => field.length === 0 || field.length > MAX_REFLOAT_FIELD_BYTES)) {
    throw new Error('Refloat field name exceeds its UTF-8 bound');
  }
  if (encoded.reduce((sum, field) => sum + field.length, 0) > MAX_REFLOAT_FIELD_NAME_BYTES) {
    throw new Error('Refloat field names exceed their aggregate UTF-8 bound');
  }
  if (new Set(all).size !== all.length) throw new Error('Refloat field layout contains duplicates');
}

function appendFieldList(bytes: number[], fields: readonly string[]): void {
  bytes.push(fields.length);
  for (const field of fields) {
    const encoded = new TextEncoder().encode(field);
    bytes.push(encoded.length, ...encoded);
  }
}

function appendFloat32Auto(destination: Uint8Array, offset: number, value: number): number {
  if (!Number.isFinite(value)) throw new Error('Float32 value must be finite');
  const view = new DataView(destination.buffer, destination.byteOffset, destination.byteLength);
  view.setFloat32(offset, Math.abs(value) < 1.5e-38 ? 0 : value, false);
  return offset + 4;
}

function packageStateCode(value: PackageState): number {
  return ['disabled', 'startup', 'ready', 'running'].indexOf(value) & 0x03;
}

function packageModeCode(value: PackageMode): number {
  return ['normal', 'handTest', 'flywheel'].indexOf(value) & 0x03;
}

function footpadStateCode(value: FootpadState): number {
  return ['none', 'left', 'right', 'both'].indexOf(value) & 0x03;
}

function stopConditionCode(value: StopCondition): number {
  return ['none', 'pitch', 'roll', 'switchHalf', 'switchFull', 'reverseStop', 'quickStop'].indexOf(value) & 0x0f;
}

function setpointAdjustmentCode(value: SetpointAdjustment): number {
  return (
    (
      {
        none: 0,
        centering: 1,
        reverseStop: 2,
        speed: 5,
        duty: 6,
        error: 7,
        highVoltage: 10,
        lowVoltage: 11,
        temperature: 12,
        unknown: 15,
      } satisfies Record<SetpointAdjustment, number>
    )[value] ?? 15
  );
}

function alertReasonCode(value: AlertReason): number {
  return (
    (
      {
        none: 0,
        lowVoltage: 1,
        highVoltage: 2,
        controllerTemperature: 3,
        motorTemperature: 4,
        current: 5,
        duty: 6,
        sensors: 7,
        lowBattery: 8,
        idle: 9,
        error: 10,
        speed: 11,
        cellUnderTemperature: 12,
        cellOverTemperature: 13,
        cellLowVoltage: 14,
        cellHighVoltage: 15,
        cellImbalance: 16,
        bmsConnection: 17,
        bmsOverTemperature: 18,
        firmwareFault: 19,
        unknown: 255,
      } satisfies Record<AlertReason, number>
    )[value] ?? 255
  );
}

export function firmwareFaultCode(value: FirmwareFault): number {
  if (value === 'unknown') return 255;
  return Math.max(
    1,
    [
      'unknown',
      'overVoltage',
      'underVoltage',
      'driver',
      'absoluteOverCurrent',
      'controllerOverTemperature',
      'motorOverTemperature',
      'gateDriverOverVoltage',
      'gateDriverUnderVoltage',
      'mcuUnderVoltage',
      'watchdogReset',
      'encoderSpi',
      'encoderSinCosBelowMinimum',
      'encoderSinCosAboveMaximum',
      'flashCorruption',
      'currentSensorOffset1',
      'currentSensorOffset2',
      'currentSensorOffset3',
      'unbalancedCurrents',
      'brake',
      'resolverLossOfTracking',
      'resolverDegradationOfSignal',
      'resolverLossOfSignal',
      'appConfigFlashCorruption',
      'motorConfigFlashCorruption',
      'encoderNoMagnet',
      'encoderMagnetTooStrong',
      'phaseFilter',
    ].indexOf(value)
  );
}

function legacyState(sample: TelemetrySample): number {
  if (sample.charging) return 14;
  switch (sample.packageState) {
    case 'disabled':
      return 15;
    case 'startup':
      return 0;
    case 'ready':
      return (
        (
          {
            none: 11,
            pitch: 6,
            roll: 7,
            switchHalf: 8,
            switchFull: 9,
            reverseStop: 12,
            quickStop: 13,
            unknown: 11,
          } satisfies Record<StopCondition, number>
        )[sample.stopCondition] ?? 11
      );
    case 'running':
      if (['speed', 'duty', 'error', 'highVoltage', 'lowVoltage', 'temperature'].includes(sample.setpointAdjustment)) {
        return 2;
      }
      if (sample.wheelslip) return 3;
      if (sample.darkride) return 4;
      if (sample.packageMode === 'flywheel') return 5;
      return 1;
    default:
      return 0;
  }
}

function legacySetpointAdjustment(value: SetpointAdjustment): number {
  return (
    (
      {
        centering: 0,
        reverseStop: 1,
        none: 2,
        speed: 3,
        duty: 3,
        error: 3,
        highVoltage: 4,
        lowVoltage: 5,
        temperature: 6,
        unknown: 0,
      } satisfies Record<SetpointAdjustment, number>
    )[value] ?? 0
  );
}

function legacyFootpad(value: FootpadState): number {
  return value === 'both' ? 2 : value === 'left' || value === 'right' ? 1 : 0;
}

function byte(value: number): number {
  return Math.max(0, Math.min(255, Math.trunc(value)));
}

function i16(destination: Uint8Array, offset: number, value: number) {
  const bounded = Math.max(-32768, Math.min(32767, Math.trunc(value)));
  destination[offset] = (bounded >> 8) & 0xff;
  destination[offset + 1] = bounded & 0xff;
}

function u32(destination: Uint8Array, offset: number, value: number) {
  const bounded = Math.max(0, Math.min(0xffffffff, Math.trunc(value))) >>> 0;
  destination[offset] = bounded >>> 24;
  destination[offset + 1] = bounded >>> 16;
  destination[offset + 2] = bounded >>> 8;
  destination[offset + 3] = bounded;
}

function pushU32(destination: number[], value: number) {
  const bytes = new Uint8Array(4);
  new DataView(bytes.buffer).setUint32(0, Math.max(0, Math.min(0xffffffff, Math.trunc(value))) >>> 0, false);
  destination.push(...bytes);
}

function pushF32(destination: number[], value: number) {
  if (!Number.isFinite(value)) throw new Error('Float32 value must be finite');
  const bytes = new Uint8Array(4);
  new DataView(bytes.buffer).setFloat32(0, value, false);
  destination.push(...bytes);
}

function pushF64(destination: number[], value: number) {
  if (!Number.isFinite(value)) throw new Error('Float64 value must be finite');
  const bytes = new Uint8Array(8);
  new DataView(bytes.buffer).setFloat64(0, value, false);
  destination.push(...bytes);
}

function clamp(value: number, minimum: number, maximum: number) {
  return Math.max(minimum, Math.min(maximum, value));
}

export function float16(value: number): number {
  if (!Number.isFinite(value)) throw new Error('Float16 value must be finite');
  const buffer = new ArrayBuffer(4);
  const view = new DataView(buffer);
  view.setFloat32(0, value, false);
  const bits = view.getUint32(0, false);
  const sign = (bits >>> 16) & 0x8000;
  let exponent = ((bits >>> 23) & 0xff) - 127 + 15;
  let fraction = bits & 0x7fffff;
  if (exponent <= 0) {
    if (exponent < -10) return sign;
    fraction = (fraction | 0x800000) >>> (1 - exponent);
    return sign | ((fraction + 0x1000) >>> 13);
  }
  if (exponent >= 31) return sign | 0x7bff;
  fraction += 0x1000;
  if (fraction & 0x800000) {
    fraction = 0;
    exponent += 1;
  }
  return sign | (exponent << 10) | (fraction >>> 13);
}
