import type { FirmwareFaultEntry, FirmwareFault } from '$lib/backend/contract';

const labels: Record<FirmwareFault, string> = {
  overVoltage: 'Overvoltage',
  underVoltage: 'Undervoltage',
  driver: 'Driver fault',
  absoluteOverCurrent: 'Absolute overcurrent',
  controllerOverTemperature: 'Controller overheating',
  motorOverTemperature: 'Motor overheating',
  gateDriverOverVoltage: 'Gate driver overvoltage',
  gateDriverUnderVoltage: 'Gate driver undervoltage',
  mcuUnderVoltage: 'MCU undervoltage',
  watchdogReset: 'Watchdog reset',
  encoderSpi: 'Encoder SPI fault',
  encoderSinCosBelowMinimum: 'Encoder signal too low',
  encoderSinCosAboveMaximum: 'Encoder signal too high',
  flashCorruption: 'Flash corruption',
  currentSensorOffset1: 'Current sensor 1 offset',
  currentSensorOffset2: 'Current sensor 2 offset',
  currentSensorOffset3: 'Current sensor 3 offset',
  unbalancedCurrents: 'Unbalanced currents',
  brake: 'Brake fault',
  resolverLossOfTracking: 'Resolver tracking lost',
  resolverDegradationOfSignal: 'Resolver signal degraded',
  resolverLossOfSignal: 'Resolver signal lost',
  appConfigFlashCorruption: 'App configuration corrupted',
  motorConfigFlashCorruption: 'Motor configuration corrupted',
  encoderNoMagnet: 'Encoder magnet missing',
  encoderMagnetTooStrong: 'Encoder magnet too strong',
  phaseFilter: 'Phase filter fault',
  unknown: 'Unknown controller fault',
};
export function faultLabel(reason: string, code: number | null): string {
  if (reason === 'imuTimeout') return 'IMU timeout';
  if (reason === 'imuTimeoutCleared') return 'IMU timeout cleared';
  return Object.hasOwn(labels, reason) && reason !== 'unknown'
    ? labels[reason as FirmwareFault]
    : code == null
      ? 'Unknown controller fault'
      : `Controller fault ${code}`;
}
export function faultTime(elapsedMs: number | null): string {
  if (elapsedMs === null) return 'Time unavailable';
  const seconds = Math.floor(elapsedMs / 1000);
  return `≈ ${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
}
export interface RideFault extends Omit<FirmwareFaultEntry, 'reason' | 'code'> {
  sequence: number;
  reason: string;
  code: number | null;
}
