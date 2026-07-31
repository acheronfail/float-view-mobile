export type PackageState = 'disabled' | 'startup' | 'ready' | 'running' | 'unknown';
export type PackageMode = 'normal' | 'handTest' | 'flywheel' | 'unknown';
export type FootpadState = 'none' | 'left' | 'right' | 'both' | 'unknown';
export type StopCondition =
  'none' | 'pitch' | 'roll' | 'switchHalf' | 'switchFull' | 'reverseStop' | 'quickStop' | 'unknown';
export type SetpointAdjustment =
  | 'none'
  | 'centering'
  | 'reverseStop'
  | 'speed'
  | 'duty'
  | 'error'
  | 'highVoltage'
  | 'lowVoltage'
  | 'temperature'
  | 'unknown';
export type AlertReason =
  | 'none'
  | 'lowVoltage'
  | 'highVoltage'
  | 'controllerTemperature'
  | 'motorTemperature'
  | 'current'
  | 'duty'
  | 'sensors'
  | 'lowBattery'
  | 'idle'
  | 'error'
  | 'speed'
  | 'cellUnderTemperature'
  | 'cellOverTemperature'
  | 'cellLowVoltage'
  | 'cellHighVoltage'
  | 'cellImbalance'
  | 'bmsConnection'
  | 'bmsOverTemperature'
  | 'firmwareFault'
  | 'unknown';
export type FirmwareFault =
  | 'overVoltage'
  | 'underVoltage'
  | 'driver'
  | 'absoluteOverCurrent'
  | 'controllerOverTemperature'
  | 'motorOverTemperature'
  | 'gateDriverOverVoltage'
  | 'gateDriverUnderVoltage'
  | 'mcuUnderVoltage'
  | 'watchdogReset'
  | 'encoderSpi'
  | 'encoderSinCosBelowMinimum'
  | 'encoderSinCosAboveMaximum'
  | 'flashCorruption'
  | 'currentSensorOffset1'
  | 'currentSensorOffset2'
  | 'currentSensorOffset3'
  | 'unbalancedCurrents'
  | 'brake'
  | 'resolverLossOfTracking'
  | 'resolverDegradationOfSignal'
  | 'resolverLossOfSignal'
  | 'appConfigFlashCorruption'
  | 'motorConfigFlashCorruption'
  | 'encoderNoMagnet'
  | 'encoderMagnetTooStrong'
  | 'phaseFilter'
  | 'unknown';

/**
 * Version-independent semantic board state. Protocol codecs project this state
 * into only the fields supported by the selected Refloat source family.
 */
export interface SimulatedBoardState {
  rideState: 'idle' | 'riding';
  speedKph: number;
  erpm: number;
  pitchDeg: number;
  balancePitchDeg: number;
  rollDeg: number;
  distanceM: number;
  motorCurrentA: number;
  directionalMotorCurrentA: number;
  filteredMotorCurrentA: number;
  batteryCurrentA: number;
  requestedBalanceCurrentA: number;
  boosterCurrentA: number;
  fieldWeakeningCurrentA: number;
  dutyCycle: number;
  packVoltageV: number;
  controllerTempC: number;
  motorTempC: number;
  footpadState: FootpadState;
  footpadAdc1V: number;
  footpadAdc2V: number;
  remoteInput: number;
  setpointDeg: number;
  atrSetpointDeg: number;
  brakeTiltSetpointDeg: number;
  torqueTiltSetpointDeg: number;
  turnTiltSetpointDeg: number;
  remoteSetpointDeg: number;
  atrAccelerationDifferenceErpmPerSecond: number;
  atrSpeedBoost: number;
  packageState: PackageState;
  packageMode: PackageMode;
  stopCondition: StopCondition;
  setpointAdjustment: SetpointAdjustment;
  wheelslip: boolean;
  darkride: boolean;
  charging: boolean;
  alertReason: AlertReason;
  firmwareFault: FirmwareFault | null;
  odometerM: number;
  ampHoursDrawnAh: number;
  ampHoursChargedAh: number;
  wattHoursDrawnWh: number;
  wattHoursChargedWh: number;
  batteryLevel: number;
  chargingCurrentA: number;
  chargingVoltageV: number;
}

export type TelemetrySample = SimulatedBoardState;
export type TelemetryScene =
  | 'idle'
  | 'engaged'
  | 'cruise'
  | 'speedSweep'
  | 'acceleration'
  | 'braking'
  | 'atr'
  | 'highCurrent'
  | 'highDuty'
  | 'fieldWeakening'
  | 'speedPushback'
  | 'dutyPushback'
  | 'highVoltagePushback'
  | 'lowVoltagePushback'
  | 'temperaturePushback'
  | 'footpadLeft'
  | 'footpadRight'
  | 'footpadBoth'
  | 'stopPitch'
  | 'stopRoll'
  | 'stopSwitchHalf'
  | 'stopSwitchFull'
  | 'reverseStop'
  | 'quickStop'
  | 'controllerHot'
  | 'motorHot'
  | 'lowVoltage'
  | 'highVoltage'
  | 'wheelslip'
  | 'darkride'
  | 'handTest'
  | 'flywheel'
  | 'charging'
  | 'firmwareFault';

export interface TelemetrySource {
  next(): TelemetrySample;
  phase(): 'idle' | 'riding';
}

const sceneBaselines: Readonly<Record<TelemetryScene, TelemetrySample>> = {
  idle: {
    rideState: 'idle',
    speedKph: 0,
    erpm: 0,
    pitchDeg: 0,
    balancePitchDeg: 0,
    rollDeg: 0,
    distanceM: 0,
    motorCurrentA: 0,
    directionalMotorCurrentA: 0,
    filteredMotorCurrentA: 0,
    batteryCurrentA: 0,
    requestedBalanceCurrentA: 0,
    boosterCurrentA: 0,
    fieldWeakeningCurrentA: 0,
    dutyCycle: 0,
    packVoltageV: 63.4,
    controllerTempC: 30,
    motorTempC: 28,
    footpadState: 'none',
    footpadAdc1V: 0.25,
    footpadAdc2V: 0.24,
    remoteInput: 0,
    setpointDeg: 0,
    atrSetpointDeg: 0,
    brakeTiltSetpointDeg: 0,
    torqueTiltSetpointDeg: 0,
    turnTiltSetpointDeg: 0,
    remoteSetpointDeg: 0,
    atrAccelerationDifferenceErpmPerSecond: 0,
    atrSpeedBoost: 0,
    packageState: 'ready',
    packageMode: 'normal',
    stopCondition: 'none',
    setpointAdjustment: 'none',
    wheelslip: false,
    darkride: false,
    charging: false,
    alertReason: 'none',
    firmwareFault: null,
    odometerM: 12_345,
    ampHoursDrawnAh: 18.4,
    ampHoursChargedAh: 2.7,
    wattHoursDrawnWh: 812,
    wattHoursChargedWh: 119,
    batteryLevel: 0.82,
    chargingCurrentA: 0,
    chargingVoltageV: 0,
  },
  engaged: riding({ speedKph: 1.5, erpm: 350, motorCurrentA: 8, dutyCycle: 0.08 }),
  cruise: riding({}),
  speedSweep: riding({ speedKph: 22, erpm: 5_170 }),
  acceleration: riding({
    speedKph: 24,
    erpm: 5_640,
    pitchDeg: 3.2,
    motorCurrentA: 48,
    directionalMotorCurrentA: 46,
    filteredMotorCurrentA: 43,
    batteryCurrentA: 25,
    requestedBalanceCurrentA: 52,
    boosterCurrentA: 6,
    dutyCycle: 0.64,
    packVoltageV: 60.8,
    torqueTiltSetpointDeg: 1.8,
    atrSetpointDeg: 2.4,
    atrAccelerationDifferenceErpmPerSecond: 1_250,
    atrSpeedBoost: 0.18,
  }),
  braking: riding({
    speedKph: 15,
    erpm: 3_525,
    pitchDeg: -2.5,
    motorCurrentA: -34,
    directionalMotorCurrentA: -32,
    filteredMotorCurrentA: -30,
    batteryCurrentA: -15,
    requestedBalanceCurrentA: -38,
    brakeTiltSetpointDeg: -2.2,
    dutyCycle: 0.38,
  }),
  atr: riding({
    atrSetpointDeg: 4.2,
    atrAccelerationDifferenceErpmPerSecond: 1_800,
    atrSpeedBoost: 0.35,
    requestedBalanceCurrentA: 42,
  }),
  highCurrent: riding({
    motorCurrentA: 78,
    directionalMotorCurrentA: 74,
    filteredMotorCurrentA: 70,
    batteryCurrentA: 39,
    requestedBalanceCurrentA: 82,
    boosterCurrentA: 9,
    packVoltageV: 58.9,
    alertReason: 'current',
  }),
  highDuty: riding({ dutyCycle: 0.91, speedKph: 31, erpm: 7_285 }),
  fieldWeakening: riding({
    dutyCycle: 0.94,
    speedKph: 35,
    erpm: 8_225,
    fieldWeakeningCurrentA: 14,
    motorCurrentA: 42,
  }),
  speedPushback: riding({
    setpointAdjustment: 'speed',
    alertReason: 'speed',
    setpointDeg: 3,
    speedKph: 32,
  }),
  dutyPushback: riding({
    setpointAdjustment: 'duty',
    alertReason: 'duty',
    setpointDeg: 3.5,
    dutyCycle: 0.92,
  }),
  highVoltagePushback: riding({
    setpointAdjustment: 'highVoltage',
    alertReason: 'highVoltage',
    packVoltageV: 67.2,
    setpointDeg: 4,
  }),
  lowVoltagePushback: riding({
    setpointAdjustment: 'lowVoltage',
    alertReason: 'lowVoltage',
    packVoltageV: 48,
    batteryLevel: 0.08,
    setpointDeg: 4,
  }),
  temperaturePushback: riding({
    setpointAdjustment: 'temperature',
    alertReason: 'controllerTemperature',
    controllerTempC: 86,
    motorTempC: 72,
    setpointDeg: 4,
  }),
  footpadLeft: riding({ footpadState: 'left', footpadAdc1V: 2.8, footpadAdc2V: 0.25 }),
  footpadRight: riding({ footpadState: 'right', footpadAdc1V: 0.25, footpadAdc2V: 2.8 }),
  footpadBoth: riding({ footpadState: 'both', footpadAdc1V: 2.8, footpadAdc2V: 2.75 }),
  stopPitch: stopped('pitch'),
  stopRoll: stopped('roll'),
  stopSwitchHalf: stopped('switchHalf'),
  stopSwitchFull: stopped('switchFull'),
  reverseStop: stopped('reverseStop'),
  quickStop: stopped('quickStop'),
  controllerHot: riding({
    controllerTempC: 88,
    alertReason: 'controllerTemperature',
    setpointAdjustment: 'temperature',
  }),
  motorHot: riding({
    motorTempC: 92,
    alertReason: 'motorTemperature',
    setpointAdjustment: 'temperature',
  }),
  lowVoltage: riding({ packVoltageV: 47, batteryLevel: 0.06, alertReason: 'lowVoltage' }),
  highVoltage: riding({ packVoltageV: 68, alertReason: 'highVoltage' }),
  wheelslip: riding({ wheelslip: true, motorCurrentA: 62, erpm: 8_000, speedKph: 12 }),
  darkride: riding({ darkride: true, rollDeg: 179, packageMode: 'normal' }),
  handTest: riding({ packageMode: 'handTest', speedKph: 0, erpm: 0, dutyCycle: 0.04 }),
  flywheel: riding({ packageMode: 'flywheel', footpadState: 'none', speedKph: 10 }),
  charging: {
    ...sceneBaselinesIdle(),
    packageState: 'ready',
    charging: true,
    chargingCurrentA: 3.5,
    chargingVoltageV: 67.1,
    batteryCurrentA: -3.5,
    batteryLevel: 0.66,
  },
  firmwareFault: {
    ...sceneBaselinesIdle(),
    alertReason: 'firmwareFault',
    firmwareFault: 'overVoltage',
    packVoltageV: 69,
  },
};

export class InteractiveTelemetrySource implements TelemetrySource {
  private scene: TelemetryScene = 'idle';
  private running = false;
  private frozen = false;
  private index = 0;
  private readonly seed: number;
  private pins: Partial<TelemetrySample> = {};
  private latest: TelemetrySample = { ...sceneBaselines.idle };

  constructor(seed = 0x5eed1234) {
    this.seed = seed >>> 0;
  }

  start(): boolean {
    if (this.running) return false;
    this.running = true;
    if (this.scene === 'idle') this.scene = 'cruise';
    this.index = 0;
    return true;
  }

  stop(): boolean {
    if (!this.running) return false;
    this.running = false;
    this.scene = 'idle';
    return true;
  }

  setScene(scene: TelemetryScene): void {
    this.scene = scene;
    this.running = scene !== 'idle';
    this.index = 0;
    this.latest = this.resolve(sceneBaselines[scene]);
  }

  freeze(): boolean {
    if (this.frozen) return false;
    this.frozen = true;
    return true;
  }

  resume(): boolean {
    if (!this.frozen) return false;
    this.frozen = false;
    return true;
  }

  pin<K extends keyof TelemetrySample>(field: K, value: TelemetrySample[K]): void {
    if (!(field in sceneBaselines.idle)) throw new Error(`Unknown telemetry field ${field}`);
    validateField(field, value);
    this.pins[field] = value;
    this.latest = this.resolve(this.latest);
  }

  unpin(field: keyof TelemetrySample): void {
    delete this.pins[field];
  }

  reset(): void {
    this.pins = {};
    this.frozen = false;
    this.index = 0;
    this.latest = this.resolve(sceneBaselines[this.scene]);
  }

  phase() {
    return this.running ? 'riding' : 'idle';
  }

  sampleIndex() {
    return this.index;
  }

  currentScene() {
    return this.scene;
  }

  isFrozen() {
    return this.frozen;
  }

  pinnedFields() {
    return { ...this.pins };
  }

  next(): TelemetrySample {
    const baseline = sceneBaselines[this.running ? this.scene : 'idle'];
    if (!this.running || this.frozen) {
      this.latest = this.resolve(this.frozen ? this.latest : baseline);
      return this.latest;
    }
    const index = this.index++;
    const slow = Math.sin(index / 8);
    const speedKph =
      this.scene === 'speedSweep'
        ? baseline.speedKph + Math.sin(index / 5) * 16
        : baseline.speedKph + slow * 0.35 + jitter(this.seed, index) * 0.08;
    const state: TelemetrySample = {
      ...baseline,
      speedKph,
      erpm: Math.round(speedKph * 235),
      pitchDeg: baseline.pitchDeg + slow * 0.12,
      dutyCycle: clamp(baseline.dutyCycle + slow * 0.01, -1, 1),
      distanceM: baseline.distanceM + (index * Math.max(0, speedKph)) / 36,
      packVoltageV: baseline.packVoltageV - Math.max(0, baseline.batteryCurrentA) * 0.002,
      controllerTempC: baseline.controllerTempC + Math.min(index, 300) * 0.005,
      motorTempC: baseline.motorTempC + Math.min(index, 300) * 0.004,
    };
    this.latest = this.resolve(state);
    return this.latest;
  }

  lastSample() {
    return this.latest;
  }

  private resolve(sample: TelemetrySample): TelemetrySample {
    const value = { ...sample, ...this.pins };
    validateState(value);
    return value;
  }
}

export const idleTelemetry: TelemetrySample = { ...sceneBaselines.idle };
export const telemetryScenes = Object.keys(sceneBaselines) as TelemetryScene[];
export const telemetryFieldNames = Object.keys(sceneBaselines.idle) as (keyof TelemetrySample)[];

function riding(overrides: Partial<TelemetrySample>): TelemetrySample {
  return {
    ...sceneBaselinesIdle(),
    rideState: 'riding',
    speedKph: 21,
    erpm: 4_935,
    pitchDeg: 1.4,
    balancePitchDeg: 1.1,
    rollDeg: 0.8,
    motorCurrentA: 31,
    directionalMotorCurrentA: 29,
    filteredMotorCurrentA: 28,
    batteryCurrentA: 14,
    requestedBalanceCurrentA: 34,
    dutyCycle: 0.52,
    packVoltageV: 62.1,
    footpadState: 'both',
    footpadAdc1V: 2.8,
    footpadAdc2V: 2.75,
    setpointDeg: 1,
    packageState: 'running',
    ...overrides,
  };
}

function stopped(stopCondition: StopCondition): TelemetrySample {
  return {
    ...sceneBaselinesIdle(),
    stopCondition,
    alertReason: stopCondition === 'none' ? 'none' : 'error',
  };
}

function sceneBaselinesIdle(): TelemetrySample {
  return {
    rideState: 'idle',
    speedKph: 0,
    erpm: 0,
    pitchDeg: 0,
    balancePitchDeg: 0,
    rollDeg: 0,
    distanceM: 0,
    motorCurrentA: 0,
    directionalMotorCurrentA: 0,
    filteredMotorCurrentA: 0,
    batteryCurrentA: 0,
    requestedBalanceCurrentA: 0,
    boosterCurrentA: 0,
    fieldWeakeningCurrentA: 0,
    dutyCycle: 0,
    packVoltageV: 63.4,
    controllerTempC: 30,
    motorTempC: 28,
    footpadState: 'none',
    footpadAdc1V: 0.25,
    footpadAdc2V: 0.24,
    remoteInput: 0,
    setpointDeg: 0,
    atrSetpointDeg: 0,
    brakeTiltSetpointDeg: 0,
    torqueTiltSetpointDeg: 0,
    turnTiltSetpointDeg: 0,
    remoteSetpointDeg: 0,
    atrAccelerationDifferenceErpmPerSecond: 0,
    atrSpeedBoost: 0,
    packageState: 'ready',
    packageMode: 'normal',
    stopCondition: 'none',
    setpointAdjustment: 'none',
    wheelslip: false,
    darkride: false,
    charging: false,
    alertReason: 'none',
    firmwareFault: null,
    odometerM: 12_345,
    ampHoursDrawnAh: 18.4,
    ampHoursChargedAh: 2.7,
    wattHoursDrawnWh: 812,
    wattHoursChargedWh: 119,
    batteryLevel: 0.82,
    chargingCurrentA: 0,
    chargingVoltageV: 0,
  };
}

function validateState(state: TelemetrySample): void {
  for (const [field, value] of Object.entries(state)) validateField(field as keyof TelemetrySample, value);
  if (Math.abs(state.remoteInput) > 1) throw new Error('remoteInput must be in [-1, 1]');
  if (Math.abs(state.dutyCycle) > 1) throw new Error('dutyCycle must be in [-1, 1]');
  if (state.batteryLevel < 0 || state.batteryLevel > 1) {
    throw new Error('batteryLevel must be in [0, 1]');
  }
}

function validateField(field: keyof TelemetrySample, value: unknown): void {
  const baseline = sceneBaselines.idle[field];
  if (field === 'firmwareFault') {
    if (value !== null && (typeof value !== 'string' || !firmwareFaultValues.has(value))) {
      throw new Error(`${field} must be a known firmware fault or null`);
    }
    return;
  }
  if (typeof baseline !== typeof value) throw new Error(`${field} has an invalid value type`);
  if (typeof value === 'number' && !Number.isFinite(value)) throw new Error(`${field} must be finite`);
  const allowed = telemetryEnumValues[field];
  if (allowed && !allowed.has(value as string)) throw new Error(`${field} has an invalid enum value`);
}

const telemetryEnumValues: Partial<Record<keyof TelemetrySample, ReadonlySet<string>>> = {
  rideState: new Set(['idle', 'riding']),
  packageState: new Set(['disabled', 'startup', 'ready', 'running', 'unknown']),
  packageMode: new Set(['normal', 'handTest', 'flywheel', 'unknown']),
  footpadState: new Set(['none', 'left', 'right', 'both', 'unknown']),
  stopCondition: new Set(['none', 'pitch', 'roll', 'switchHalf', 'switchFull', 'reverseStop', 'quickStop', 'unknown']),
  setpointAdjustment: new Set([
    'none',
    'centering',
    'reverseStop',
    'speed',
    'duty',
    'error',
    'highVoltage',
    'lowVoltage',
    'temperature',
    'unknown',
  ]),
  alertReason: new Set([
    'none',
    'lowVoltage',
    'highVoltage',
    'controllerTemperature',
    'motorTemperature',
    'current',
    'duty',
    'sensors',
    'lowBattery',
    'idle',
    'error',
    'speed',
    'cellUnderTemperature',
    'cellOverTemperature',
    'cellLowVoltage',
    'cellHighVoltage',
    'cellImbalance',
    'bmsConnection',
    'bmsOverTemperature',
    'firmwareFault',
    'unknown',
  ]),
};

const firmwareFaultValues = new Set<string>([
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
  'unknown',
]);

function jitter(seed: number, index: number): number {
  let value = (seed + Math.imul(index + 1, 0x9e3779b1)) >>> 0;
  value ^= value << 13;
  value ^= value >>> 17;
  value ^= value << 5;
  return ((value >>> 0) / 0xffffffff) * 2 - 1;
}

function clamp(value: number, minimum: number, maximum: number) {
  return Math.max(minimum, Math.min(maximum, value));
}
