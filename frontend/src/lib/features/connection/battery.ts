import type { BatteryEstimate } from '$lib/backend/contract';

/** Round once, identically for text, gauge and accessibility. No estimation here. */
export function estimatedBatteryPercent(estimate: BatteryEstimate | null | undefined): number | null {
  return estimate?.level == null ? null : Math.round(estimate.level * 100);
}
