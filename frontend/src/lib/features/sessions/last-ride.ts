import type { SavedDevice } from '$lib/backend/contract';

export function latestRideAt(devices: readonly SavedDevice[]): number | null {
  let latest: number | null = null;
  for (const device of devices) {
    const timestamp = Date.parse(device.summary.mostRecentSessionAt ?? '');
    if (Number.isFinite(timestamp) && (latest === null || timestamp > latest)) latest = timestamp;
  }
  return latest;
}

export function lastRideLabel(timestamp: number | null, now: number): string {
  if (timestamp === null) return 'No rides recorded yet';
  const date = new Date(timestamp);
  const elapsed = now - timestamp;
  const relative = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' });
  if (elapsed >= 0 && elapsed < 60_000) return 'Last ride just now';
  if (elapsed >= 60_000 && elapsed < 3_600_000) {
    return `Last ride ${relative.format(-Math.floor(elapsed / 60_000), 'minute')}`;
  }

  const today = new Date(now);
  // Compare local calendar dates, not 24-hour intervals: DST days vary in length.
  const calendarDay = (value: Date) => Date.UTC(value.getFullYear(), value.getMonth(), value.getDate());
  const days = Math.round((calendarDay(date) - calendarDay(today)) / 86_400_000);
  const day =
    days <= 0 && days >= -6
      ? relative.format(days, 'day')
      : new Intl.DateTimeFormat(undefined, {
          month: 'short',
          day: 'numeric',
          ...(date.getFullYear() !== today.getFullYear() ? { year: 'numeric' as const } : {}),
        }).format(date);
  const time = new Intl.DateTimeFormat(undefined, { hour: 'numeric', minute: '2-digit' }).format(date);
  return `Last ride ${day} at ${time}`;
}
