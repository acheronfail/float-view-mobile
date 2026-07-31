const DISMISS_DISTANCE_RATIO = 0.06;
const MINIMUM_DISMISS_DISTANCE_PX = 24;
const DISMISS_VELOCITY_PX_PER_MS = 0.5;
const MINIMUM_FLING_DISTANCE_PX = 8;

export interface SheetRelease {
  distancePx: number;
  heightPx: number;
  velocityPxPerMs: number;
}

export function shouldDismissSheet({ distancePx, heightPx, velocityPxPerMs }: SheetRelease): boolean {
  const distanceThreshold = Math.max(MINIMUM_DISMISS_DISTANCE_PX, heightPx * DISMISS_DISTANCE_RATIO);
  return (
    distancePx >= distanceThreshold ||
    (distancePx >= MINIMUM_FLING_DISTANCE_PX && velocityPxPerMs >= DISMISS_VELOCITY_PX_PER_MS)
  );
}
