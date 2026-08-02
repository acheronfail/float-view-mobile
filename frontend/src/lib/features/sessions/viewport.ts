export interface Viewport {
  startElapsedMs: number;
  endElapsedMs: number;
}

export function clampViewport(viewport: Viewport, durationMs: number): Viewport {
  const duration = Math.max(1, Math.round(durationMs));
  const span = Math.max(1, Math.min(duration, Math.round(viewport.endElapsedMs - viewport.startElapsedMs)));
  const start = Math.max(0, Math.min(duration - span, Math.round(viewport.startElapsedMs)));
  return { startElapsedMs: start, endElapsedMs: start + span };
}

export function panViewport(viewport: Viewport, deltaMs: number, durationMs: number): Viewport {
  const span = viewport.endElapsedMs - viewport.startElapsedMs;
  return clampViewport(
    { startElapsedMs: viewport.startElapsedMs + deltaMs, endElapsedMs: viewport.startElapsedMs + deltaMs + span },
    durationMs
  );
}

export function pinchViewport(
  viewport: Viewport,
  initialDistancePx: number,
  nextDistancePx: number,
  midpointRatio: number,
  durationMs: number
): Viewport {
  if (
    !Number.isFinite(initialDistancePx) ||
    initialDistancePx <= 0 ||
    !Number.isFinite(nextDistancePx) ||
    nextDistancePx <= 0
  )
    return clampViewport(viewport, durationMs);
  const span = viewport.endElapsedMs - viewport.startElapsedMs;
  const nextSpan = (span * initialDistancePx) / nextDistancePx;
  const midpoint = viewport.startElapsedMs + span * Math.max(0, Math.min(1, midpointRatio));
  return clampViewport(
    {
      startElapsedMs: midpoint - nextSpan * Math.max(0, Math.min(1, midpointRatio)),
      endElapsedMs: midpoint + nextSpan * (1 - Math.max(0, Math.min(1, midpointRatio))),
    },
    durationMs
  );
}
