import type { SavedDevice } from '$lib/backend/contract';

export function totalSessionStorageBytes(devices: SavedDevice[]): number | null {
  let total = 0;
  for (const device of devices) {
    const bytes = device.summary.totalStorageBytes;
    if (bytes === null) return null;
    total += bytes;
  }
  return total;
}

export function formatStorageBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(1)} ${units[unit]}`;
}
