export function suggestedPruneResolution(existingResolutionSeconds: number | null): number {
  if (!existingResolutionSeconds) return 5;
  return Math.min(300, existingResolutionSeconds * 2);
}

export function hasCoarserPruneResolution(existingResolutionSeconds: number | null): boolean {
  return !existingResolutionSeconds || existingResolutionSeconds * 2 <= 300;
}

export function validPruneResolution(value: number, existingResolutionSeconds: number | null): boolean {
  if (!Number.isSafeInteger(value) || value < 1 || value > 300) return false;
  if (!existingResolutionSeconds) return true;
  return value > existingResolutionSeconds && value % existingResolutionSeconds === 0;
}

export function formatPruneBytes(value: number | null): string {
  if (value === null) return 'Storage estimate unavailable';
  if (value < 1024) return `${value} B reclaimable`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB reclaimable`;
  return `${(value / (1024 * 1024)).toFixed(1)} MB reclaimable`;
}
