import type { SessionLocationPoint } from '$lib/backend/contract';
import type { Viewport } from './viewport';

export interface MapBounds {
  west: number;
  east: number;
  south: number;
  north: number;
}
export const MAX_MAP_INTERVALS = 128;
const mercator = (latitude: number) =>
  Math.log(Math.tan(Math.PI / 4 + (Math.max(-85.05112878, Math.min(85.05112878, latitude)) * Math.PI) / 360));
export const unwrapLongitude = (longitude: number, center: number) =>
  longitude + 360 * Math.round((center - longitude) / 360);

/** Clip continuous edges; hold the last fix during gaps for chart selection only. */
export function mapIntervals(
  segments: readonly (readonly SessionLocationPoint[])[],
  bounds: MapBounds,
  endElapsedMs = segments.at(-1)?.at(-1)?.elapsedMs ?? 0
): Viewport[] {
  const result: Viewport[] = [];
  const east = bounds.east < bounds.west ? bounds.east + 360 : bounds.east;
  const center = (bounds.west + east) / 2;
  const south = mercator(bounds.south),
    north = mercator(bounds.north);
  function include(startElapsedMs: number, endElapsedMs: number) {
    if (endElapsedMs <= startElapsedMs) return;
    const previous = result.at(-1);
    if (previous && previous.endElapsedMs === startElapsedMs) previous.endElapsedMs = endElapsedMs;
    else result.push({ startElapsedMs, endElapsedMs });
    if (result.length > MAX_MAP_INTERVALS)
      throw new Error('Too many visible ride segments (maximum 128). Zoom out or disable map filtering.');
  }
  function hold(point: SessionLocationPoint | undefined, until: number) {
    if (!point) return;
    const longitude = unwrapLongitude(point.longitude, center);
    if (
      longitude >= bounds.west &&
      longitude <= east &&
      point.latitude >= bounds.south &&
      point.latitude <= bounds.north
    )
      include(point.elapsedMs, until);
  }
  let lastFix: SessionLocationPoint | undefined;
  for (const segment of segments) {
    if (!segment.length) continue;
    hold(lastFix, segment[0]!.elapsedMs);
    for (let i = 1; i < segment.length; i++) {
      const a = segment[i - 1]!,
        b = segment[i]!;
      if (b.elapsedMs <= a.elapsedMs) continue;
      let x = unwrapLongitude(a.longitude, center);
      let endX = unwrapLongitude(b.longitude, x);
      const shift = 360 * Math.round((center - (x + endX) / 2) / 360);
      x += shift;
      endX += shift;
      const y = mercator(a.latitude),
        dx = endX - x,
        dy = mercator(b.latitude) - y;
      let enter = 0,
        exit = 1;
      for (const [p, q] of [
        [-dx, x - bounds.west],
        [dx, east - x],
        [-dy, y - south],
        [dy, north - y],
      ]) {
        if (p === 0) {
          if (q! < 0) {
            exit = -1;
            break;
          }
        } else if (p! < 0) enter = Math.max(enter, q! / p!);
        else exit = Math.min(exit, q! / p!);
      }
      if (exit <= enter) {
        continue;
      }
      const duration = b.elapsedMs - a.elapsedMs;
      // The session-data protocol uses integer milliseconds. Stay inside the clip.
      const startElapsedMs = Math.ceil(a.elapsedMs + enter * duration);
      const endElapsedMs = Math.floor(a.elapsedMs + exit * duration);
      if (endElapsedMs <= startElapsedMs) {
        continue;
      }
      include(startElapsedMs, endElapsedMs);
    }
    lastFix = segment.at(-1);
  }
  hold(lastFix, endElapsedMs);
  return result;
}

export function timeline(intervals: readonly Viewport[]) {
  let duration = 0;
  const pieces = intervals
    .filter((r) => r.endElapsedMs > r.startElapsedMs)
    .map((range) => {
      const offset = duration;
      duration += range.endElapsedMs - range.startElapsedMs;
      return { ...range, offset };
    });
  const indexAt = (elapsed: number) =>
    pieces.findIndex((r) => elapsed >= r.startElapsedMs && elapsed <= r.endElapsedMs);
  return {
    duration,
    separators: pieces.slice(1).map((r) => r.offset / duration),
    indexAt,
    ratioAt(elapsed: number): number | null {
      const r = pieces[indexAt(elapsed)];
      return r ? (r.offset + elapsed - r.startElapsedMs) / duration : null;
    },
    elapsedAt(ratio: number): number | null {
      const offset = Math.max(0, Math.min(1, ratio)) * duration;
      const r = pieces.find((r) => offset < r.offset + r.endElapsedMs - r.startElapsedMs) ?? pieces.at(-1);
      return r ? r.startElapsedMs + offset - r.offset : null;
    },
  };
}

export function intervalBuckets(intervals: readonly Viewport[], width: number): number[] {
  if (!intervals.length) return [];
  const total = Math.min(1024, Math.max(intervals.length, Math.floor(width)));
  const duration = intervals.reduce((sum, r) => sum + r.endElapsedMs - r.startElapsedMs, 0);
  const shares = intervals.map((r) => ((total - intervals.length) * (r.endElapsedMs - r.startElapsedMs)) / duration);
  const buckets = shares.map((s) => 1 + Math.floor(s));
  const order = shares.map((s, i) => ({ i, fraction: s % 1 })).sort((a, b) => b.fraction - a.fraction);
  for (let left = total - buckets.reduce((a, b) => a + b, 0), i = 0; i < left; i++) buckets[order[i]!.i]!++;
  return buckets;
}
