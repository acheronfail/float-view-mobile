import type { SessionLocationPoint } from './contract.ts';

/** Keep CSV matching identical to both native stores. */
export const CSV_LOCATION_TOLERANCE_MS = 10_000;

export interface LocationProjectionRow {
  elapsedMs: number;
  locationSequence: number;
  accuracyM: number | null;
  latitude: number;
  longitude: number;
}

export interface CsvLocationColumns {
  accuracy: string;
  longitude: string;
  latitude: string;
}

/**
 * Pick one accepted location for a telemetry row. Equal-time observations use
 * the lower location sequence, matching the native SQL ORDER BY contract.
 */
export function nearestLocation(
  elapsedMs: number,
  points: readonly Pick<
    SessionLocationPoint,
    'locationSequence' | 'elapsedMs' | 'accuracyM' | 'latitude' | 'longitude'
  >[],
  toleranceMs = CSV_LOCATION_TOLERANCE_MS
): LocationProjectionRow | null {
  let best: LocationProjectionRow | null = null;
  let bestDistance = Number.POSITIVE_INFINITY;
  for (const point of points) {
    const distance = Math.abs(point.elapsedMs - elapsedMs);
    if (distance > toleranceMs) continue;
    if (
      best === null ||
      distance < bestDistance ||
      (distance === bestDistance && point.locationSequence < best.locationSequence)
    ) {
      best = { ...point };
      bestDistance = distance;
    }
  }
  return best;
}

export function csvLocationColumns(
  elapsedMs: number,
  points: readonly Pick<
    SessionLocationPoint,
    'locationSequence' | 'elapsedMs' | 'accuracyM' | 'latitude' | 'longitude'
  >[],
  toleranceMs = CSV_LOCATION_TOLERANCE_MS
): CsvLocationColumns {
  const point = nearestLocation(elapsedMs, points, toleranceMs);
  if (!point) return { accuracy: '', longitude: '', latitude: '' };
  return {
    accuracy: point.accuracyM === null ? '' : String(point.accuracyM),
    longitude: String(point.longitude),
    latitude: String(point.latitude),
  };
}
