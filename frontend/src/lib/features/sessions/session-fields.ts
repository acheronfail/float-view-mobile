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

export const sessionFieldMetadata = {
  packVoltageV: {
    displayName: 'Pack voltage',
    unit: 'V',
  },
  dutyCycle: {
    displayName: 'Duty cycle',
    unit: '%',
  },
  controllerTempC: {
    displayName: 'Controller temp',
    unit: '°C',
  },
  motorTempC: {
    displayName: 'Motor temp',
    unit: '°C',
  },
  motorCurrentA: {
    displayName: 'Motor current',
    unit: 'A',
  },
  batteryCurrentA: {
    displayName: 'Battery current',
    unit: 'A',
  },
  erpm: {
    displayName: 'ERPM',
    unit: 'electrical rpm',
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
} satisfies Record<
  StableSessionFieldName,
  {
    displayName: string;
    unit: string;
    // TODO: colors for each field, use --vars so they work across light/dark theme
    // color: string;
    // TODO: description for each field
    // description: string;
  }
>;

export function sessionFieldDisplayName(name: string): string {
  return name in sessionFieldMetadata ? sessionFieldMetadata[name as StableSessionFieldName].displayName : name;
}

export function sessionFieldUnit(name: string): string | null {
  return name in sessionFieldMetadata ? sessionFieldMetadata[name as StableSessionFieldName].unit : null;
}
