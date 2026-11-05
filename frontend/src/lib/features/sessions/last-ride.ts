import type { SavedDevice } from '$lib/backend/contract';

export function latestRideBoard(devices: readonly SavedDevice[]): SavedDevice | null {
  let board: SavedDevice | null = null;
  let latest: number | null = null;
  for (const device of devices) {
    const timestamp = Date.parse(device.summary.mostRecentSessionAt ?? '');
    if (Number.isFinite(timestamp) && (latest === null || timestamp > latest)) {
      latest = timestamp;
      board = device;
    }
  }
  return board;
}

export function latestRideAt(devices: readonly SavedDevice[]): number | null {
  const board = latestRideBoard(devices);
  return board ? Date.parse(board.summary.mostRecentSessionAt!) : null;
}

export function lastRideLabel(timestamp: number | null, now: number): string {
  if (timestamp === null) return 'No rides recorded yet';
  const elapsed = Math.max(0, now - timestamp);
  if (elapsed < 60_000) return 'Last ride was just now';
  const relative = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' });
  let amount: number;
  let unit: Intl.RelativeTimeFormatUnit;
  if (elapsed < 3_600_000) {
    amount = Math.floor(elapsed / 60_000);
    unit = 'minute';
  } else if (elapsed < 86_400_000) {
    amount = Math.floor(elapsed / 3_600_000);
    unit = 'hour';
  } else {
    // Calendar dates keep day labels natural across midnight and daylight saving.
    const calendarDay = (value: number) => {
      const date = new Date(value);
      return Date.UTC(date.getFullYear(), date.getMonth(), date.getDate());
    };
    const days = Math.round((calendarDay(now) - calendarDay(timestamp)) / 86_400_000);
    if (days < 7) {
      amount = days;
      unit = 'day';
    } else if (days < 30) {
      amount = Math.floor(days / 7);
      unit = 'week';
    } else if (days < 365) {
      amount = Math.floor(days / 30);
      unit = 'month';
    } else {
      amount = Math.floor(days / 365);
      unit = 'year';
    }
  }
  return `Last ride was ${relative.format(-amount, unit)}`;
}
