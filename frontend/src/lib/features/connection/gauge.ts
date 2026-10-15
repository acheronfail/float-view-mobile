import type { LiveSample } from '../../backend/contract';

export type DriveLoadSource = 'duty' | 'motor' | 'battery';
export interface DriveLoad {
  source: DriveLoadSource;
  percentage: number;
}
/** Presentation only: all utilisation and winner policy belongs to Rust. */
export function driveLoadComponents(sample: Pick<LiveSample, 'load'> | null | undefined) {
  return {
    duty: sample?.load.duty == null ? null : sample.load.duty * 100,
    motor: sample?.load.motor == null ? null : sample.load.motor * 100,
    battery: sample?.load.battery == null ? null : sample.load.battery * 100,
  };
}
export function driveLoad(sample: Pick<LiveSample, 'load'> | null | undefined): DriveLoad | null {
  const winner = sample?.load.winner;
  return winner ? { source: winner.source, percentage: winner.fraction * 100 } : null;
}
export function driveLoadPercent(sample: Pick<LiveSample, 'load'> | null | undefined): number | null {
  return driveLoad(sample)?.percentage ?? null;
}
