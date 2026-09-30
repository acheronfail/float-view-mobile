import type { BatteryEstimate } from '$lib/backend/contract';

/** Round once, identically for text, gauge and accessibility. No estimation here. */
export function estimatedBatteryPercent(estimate: BatteryEstimate | null | undefined): number | null {
  return estimate?.level == null ? null : Math.round(estimate.level * 100);
}

/** Select display data only; the service remains the estimator. */
export function batteryPresentation(
  estimate: BatteryEstimate | null | undefined,
  reported: number | null | undefined,
  savedBoard: boolean
) {
  const estimated = estimatedBatteryPercent(estimate);
  const needsSetup = estimated == null && savedBoard && estimate?.unavailableReason === 'not_configured';
  const validReported = reported != null && Number.isFinite(reported) && reported >= 0 && reported <= 1;
  const percent = estimated ?? (needsSetup && validReported ? Math.round(reported * 100) : null);
  return {
    percent,
    needsSetup,
    label: needsSetup ? 'Controller-reported battery level' : 'Estimated battery level',
  };
}
