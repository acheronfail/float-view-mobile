import type { BmsConnection, BmsState } from '$lib/backend/contract';

export function bmsMenuVisible(state: BmsState | null | undefined): boolean {
  if (!state || state.status === 'not_found') return false;
  return state.connection !== null || state.candidates.length > 0 || state.status !== 'discovering';
}

export function bmsRouteLabel(connection: BmsConnection | null): string {
  if (!connection) return 'Not selected';
  return connection.kind === 'direct' ? 'Direct Bluetooth endpoint' : `CAN ${connection.canId}`;
}

export function bmsFreshness(observedAt: string, nowMs: number): string {
  const ageSeconds = Math.max(0, Math.floor((nowMs - Date.parse(observedAt)) / 1000));
  if (ageSeconds < 2) return 'just now';
  if (ageSeconds < 60) return `${ageSeconds}s ago`;
  return `${Math.floor(ageSeconds / 60)}m ago`;
}

// BMS polling is once per second. Allow missed polls, but expire retained values.
export const BMS_MAX_AGE_MS = 5_000;

export function freshBmsSnapshot(state: BmsState | null | undefined, nowMs: number) {
  if (state?.status !== 'attached' || !state.connection || !state.latestSnapshot) return null;
  const age = nowMs - Date.parse(state.latestSnapshot.observedAt);
  return age >= 0 && age < BMS_MAX_AGE_MS ? state.latestSnapshot : null;
}
