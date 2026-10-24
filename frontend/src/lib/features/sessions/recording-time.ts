import type { SessionRecordingBreak } from '$lib/backend/contract';
import type { Viewport } from './viewport';
import { timeline } from './map-filter';

/** Display time only. Persisted times, requests and callbacks stay in ride time. */
export function compactElapsed(elapsed: number, breaks: readonly SessionRecordingBreak[]): number {
  return (
    elapsed -
    breaks.reduce(
      (sum, [start, end, reason]) => sum + (reason === 'pause' ? Math.max(0, Math.min(elapsed, end) - start) : 0),
      0
    )
  );
}

export function originalElapsed(elapsed: number, breaks: readonly SessionRecordingBreak[]): number {
  let removed = 0;
  for (const [start, end, reason] of breaks) {
    if (reason !== 'pause') continue;
    if (elapsed < start - removed) break;
    removed += end - start;
  }
  return elapsed + removed;
}

export function compactViewport(viewport: Viewport, breaks: readonly SessionRecordingBreak[]): Viewport {
  return {
    startElapsedMs: compactElapsed(viewport.startElapsedMs, breaks),
    endElapsedMs: compactElapsed(viewport.endElapsedMs, breaks),
  };
}

export function originalViewport(viewport: Viewport, breaks: readonly SessionRecordingBreak[]): Viewport {
  return {
    startElapsedMs: originalElapsed(viewport.startElapsedMs, breaks),
    endElapsedMs: originalElapsed(viewport.endElapsedMs, breaks),
  };
}

export function recordingTimeline(
  viewport: Viewport,
  breaks: readonly SessionRecordingBreak[] = [],
  intervals?: readonly Viewport[]
) {
  breaks = breaks.filter(([, , reason]) => reason === 'pause');
  // Build the index once per viewport, rather than scanning every break for
  // every retained point in every lazily loaded chart.
  const ends = breaks.map(([, end]) => end);
  const removed = [0];
  const seams = breaks.map(([start, end], index) => {
    const seam = start - removed[index]!;
    removed.push(removed[index]! + end - start);
    return seam;
  });
  const after = (values: readonly number[], value: number) => {
    let left = 0,
      right = values.length;
    while (left < right) {
      const middle = (left + right) >>> 1;
      if (values[middle]! <= value) left = middle + 1;
      else right = middle;
    }
    return left;
  };
  const compactAt = (elapsed: number) => {
    const index = after(ends, elapsed);
    const next = breaks[index];
    return elapsed - removed[index]! - (next ? Math.max(0, elapsed - next[0]) : 0);
  };
  const ranges = intervals ?? [viewport];
  const retained = ranges.flatMap((range) => {
    const pieces: Viewport[] = [];
    let start = range.startElapsedMs;
    for (const [left, right] of breaks) {
      if (right <= start || left >= range.endElapsedMs) continue;
      if (left > start) pieces.push({ startElapsedMs: start, endElapsedMs: left });
      start = Math.max(start, right);
    }
    if (start < range.endElapsedMs) pieces.push({ startElapsedMs: start, endElapsedMs: range.endElapsedMs });
    return pieces;
  });
  const filtered = timeline(retained);
  const compact = {
    startElapsedMs: compactAt(viewport.startElapsedMs),
    endElapsedMs: compactAt(viewport.endElapsedMs),
  };
  const duration = intervals ? filtered.duration : compact.endElapsedMs - compact.startElapsedMs;
  const indexAt = (elapsed: number) => {
    const index = after(ends, elapsed);
    const next = breaks[index];
    if (next && elapsed > next[0]) return -1;
    return intervals ? filtered.indexAt(elapsed) : index;
  };
  const ratioAt = (elapsed: number): number | null => {
    if (duration <= 0 || indexAt(elapsed) < 0) return null;
    return intervals ? filtered.ratioAt(elapsed) : (compactAt(elapsed) - compact.startElapsedMs) / duration;
  };
  return {
    duration,
    indexAt,
    ratioAt,
    elapsedAt(ratio: number): number | null {
      if (duration <= 0) return null;
      if (intervals) return filtered.elapsedAt(ratio);
      if (ratio <= 0) return viewport.startElapsedMs;
      if (ratio >= 1) return viewport.endElapsedMs;
      const elapsed = compact.startElapsedMs + ratio * duration;
      return elapsed + removed[after(seams, elapsed)]!;
    },
    // Map exclusions retain their existing separators, separately from recording breaks.
    separators: intervals
      ? timeline(intervals)
          .separators.map((_, i) => ratioAt(intervals[i + 1]!.startElapsedMs))
          .filter((r): r is number => r !== null)
      : [],
    recordingSeparators: breaks.flatMap(([start, end]) => {
      if (!ranges.some((r) => start >= r.startElapsedMs && end <= r.endElapsedMs)) return [];
      const ratio = ratioAt(start);
      return ratio === null ? [] : [{ start, end, ratio }];
    }),
  };
}
