import type { ActiveConnection } from '$lib/backend/contract';

// BMS polling is once per second. Allow missed polls, but expire retained values.
export const BMS_CHARGING_MAX_AGE_MS = 5_000;

export function liveCharging(
  connection: Pick<ActiveConnection, 'state' | 'latestSample' | 'bms'> | null | undefined,
  nowMs: number
): boolean {
  if (connection?.state !== 'ready') return false;
  if (connection.latestSample?.charging === true) return true;
  const bms = connection.bms;
  const snapshot = bms.latestSnapshot;
  if (bms.status !== 'attached' || !bms.connection || !snapshot) return false;
  const age = nowMs - Date.parse(snapshot.observedAt);
  return age >= 0 && age < BMS_CHARGING_MAX_AGE_MS && snapshot.status === 'CHARGING';
}
