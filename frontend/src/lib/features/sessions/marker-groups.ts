import type { Viewport } from './viewport';
import { recordingTimeline } from './recording-time';
import type { SessionRecordingBreak } from '$lib/backend/contract';
export function chartMarkerGroups<T extends { sequence: number; elapsedMs: number }>(
  markers: readonly T[],
  viewport: Viewport,
  width: number,
  intervals?: readonly Viewport[],
  recordingBreaks?: readonly SessionRecordingBreak[]
) {
  const groups = new Map<number, { marker: T; count: number; percent: number }>();
  const axis = recordingTimeline(viewport, recordingBreaks, intervals);
  for (const marker of markers) {
    if (!intervals && (marker.elapsedMs < viewport.startElapsedMs || marker.elapsedMs > viewport.endElapsedMs))
      continue;
    const ratio = axis.ratioAt(marker.elapsedMs);
    if (ratio === null) continue;
    const percent = ratio * 100;
    const bucket = Math.round(((percent / 100) * Math.max(1, width)) / 12);
    const group = groups.get(bucket);
    if (group) group.count++;
    else groups.set(bucket, { marker, count: 1, percent });
  }
  return [...groups.values()];
}
