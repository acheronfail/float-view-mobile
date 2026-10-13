import type { ActiveConnection } from '$lib/backend/contract';

import { freshBmsSnapshot } from './bms';

export function liveCharging(
  connection: Pick<ActiveConnection, 'state' | 'latestSample' | 'bms'> | null | undefined,
  nowMs: number
): boolean {
  if (connection?.state !== 'ready') return false;
  if (connection.latestSample?.charging === true) return true;
  return freshBmsSnapshot(connection.bms, nowMs)?.status === 'CHARGING';
}
