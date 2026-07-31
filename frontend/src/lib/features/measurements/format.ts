import type { UnitPreference } from '$lib/backend/contract';

export function formatSpeedKph(value: number | null | undefined, units: UnitPreference): string {
  if (value == null) return '—';
  return `${valueInUnits(value, units === 'metric' ? 1 : 0.621371).toFixed(1)} ${units === 'metric' ? 'km/h' : 'mph'}`;
}

export function formatDistanceM(value: number | null | undefined, units: UnitPreference): string {
  if (value == null) return '—';
  return units === 'metric' ? `${(value / 1000).toFixed(2)} km` : `${(value / 1609.344).toFixed(2)} mi`;
}

export function formatTemperatureC(value: number | null | undefined, units: UnitPreference): string {
  if (value == null) return '—';
  return units === 'metric' ? `${value.toFixed(1)} °C` : `${(value * 1.8 + 32).toFixed(1)} °F`;
}

function valueInUnits(value: number, multiplier: number): number {
  return value * multiplier;
}
