import { describe, expect, it } from 'vitest';
import type { SavedDevice } from '$lib/backend/contract';
import { lastRideLabel, latestRideAt, latestRideBoard } from './last-ride';

const board = (date: string | null) => ({ summary: { mostRecentSessionAt: date } }) as SavedDevice;

describe('last ride subtitle', () => {
  it('selects the latest ride across boards and ignores missing or invalid dates', () => {
    const latest = board('2026-02-01T00:00:00Z');
    expect(latestRideBoard([board(null), board('invalid')])).toBeNull();
    expect(latestRideBoard([board('2026-01-01T00:00:00Z'), latest, board(null)])).toBe(latest);
    expect(latestRideAt([board(null), board('invalid')])).toBeNull();
    expect(latestRideAt([])).toBeNull();
    expect(latestRideAt([board('2026-01-01T00:00:00Z'), board('2026-02-01T00:00:00Z'), board(null)])).toBe(
      Date.parse('2026-02-01T00:00:00Z')
    );
  });

  it('handles empty history and advances from just now to minutes ago', () => {
    const now = Date.now();
    expect(lastRideLabel(null, now)).toBe('No rides recorded yet');
    expect(lastRideLabel(now - 30_000, now)).toBe('Last ride just now');
    expect(lastRideLabel(now - 300_000, now)).toBe('Last ride 5 minutes ago');
  });

  it('uses local calendar days and local clock formatting across midnight', () => {
    const ride = new Date(2026, 9, 9, 23, 30);
    const now = new Date(2026, 9, 10, 1, 0);
    const time = new Intl.DateTimeFormat(undefined, { hour: 'numeric', minute: '2-digit' }).format(ride);
    expect(lastRideLabel(ride.getTime(), now.getTime())).toBe(`Last ride yesterday at ${time}`);
  });

  it('includes the year for older rides and never calls a future ride just now', () => {
    const now = new Date(2026, 9, 10, 12).getTime();
    expect(lastRideLabel(new Date(2025, 1, 1, 12).getTime(), now)).toContain('2025');
    expect(lastRideLabel(now + 60_000, now)).toContain('today at');
  });
});
