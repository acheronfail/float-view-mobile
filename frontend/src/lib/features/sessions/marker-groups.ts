import type { SessionRideMarker } from '$lib/backend/contract';
import type { Viewport } from './viewport';
export function chartMarkerGroups(markers: readonly SessionRideMarker[], viewport: Viewport, width: number) {
  const groups = new Map<number, { marker: SessionRideMarker; count: number; percent: number }>();
  const span = Math.max(1, viewport.endElapsedMs - viewport.startElapsedMs);
  for (const marker of markers) {
    if (marker.elapsedMs < viewport.startElapsedMs || marker.elapsedMs > viewport.endElapsedMs) continue;
    const percent = ((marker.elapsedMs - viewport.startElapsedMs) / span) * 100;
    const bucket = Math.round(((percent / 100) * Math.max(1, width)) / 12);
    const group = groups.get(bucket);
    if (group) group.count++;
    else groups.set(bucket, { marker, count: 1, percent });
  }
  return [...groups.values()];
}
