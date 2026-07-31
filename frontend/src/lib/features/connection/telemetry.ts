import type { LiveSample } from '$lib/backend/contract';

export const currentTelemetryFields = [
  { key: 'motorCurrentA', label: 'Actual motor current' },
  { key: 'directionalMotorCurrentA', label: 'Directional motor current' },
  { key: 'filteredMotorCurrentA', label: 'Filtered motor current' },
  { key: 'batteryCurrentA', label: 'Battery current' },
  { key: 'requestedBalanceCurrentA', label: 'Requested balance current' },
  { key: 'boosterCurrentA', label: 'Booster current' },
  { key: 'fieldWeakeningCurrentA', label: 'Field-weakening current' },
] as const satisfies readonly {
  key: keyof LiveSample;
  label: string;
}[];

export function telemetryState(value: string | null | undefined): string {
  if (value == null) return 'Unavailable';
  return value.replace(/([a-z])([A-Z])/g, '$1 $2').replace(/^./, (character) => character.toUpperCase());
}

export function telemetryFlag(value: boolean | null | undefined): string {
  if (value == null) return 'Unavailable';
  return value ? 'Active' : 'Inactive';
}

export function slowTelemetryAge(sample: Pick<LiveSample, 'observedAt' | 'slowObservedAt'> | null | undefined): string {
  if (!sample?.slowObservedAt) return 'Unavailable';
  const ageMs = Date.parse(sample.observedAt) - Date.parse(sample.slowObservedAt);
  if (!Number.isFinite(ageMs) || ageMs < 0) return 'Unavailable';
  return ageMs < 1_000 ? `${Math.round(ageMs)} ms old` : `${(ageMs / 1_000).toFixed(1)} s old`;
}

export function extensionTelemetry(sample: Pick<LiveSample, 'extensions'> | null | undefined): [string, string][] {
  if (!sample) return [];
  return Object.entries(sample.extensions)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([field, value]) => [field, Number.isFinite(value) ? value.toString() : 'Unavailable']);
}
