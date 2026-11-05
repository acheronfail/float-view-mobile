import { describe, expect, it } from 'vitest';
import type { SavedDevice } from '$lib/backend/contract';
import { lastRideLabel, latestRideAt, latestRideBoard } from './last-ride';

const board = (date: string | null) => ({ summary: { mostRecentSessionAt: date } }) as SavedDevice;

describe('latest ride board', () => {
  it('selects the latest ride across boards and ignores missing or invalid dates', () => {
    const latest = board('2026-02-01T00:00:00Z');
    expect(latestRideBoard([board(null), board('invalid')])).toBeNull();
    expect(latestRideBoard([board('2026-01-01T00:00:00Z'), latest, board(null)])).toBe(latest);
    expect(latestRideBoard([])).toBeNull();
  });
});

describe('last ride relative time', () => {
  const now = new Date(2026, 10, 5, 12).getTime();
  it.each([
    [0, 'just now'],
    [30_000, 'just now'],
    [60_000, '1 minute ago'],
    [5 * 60_000, '5 minutes ago'],
    [3_600_000, '1 hour ago'],
    [3 * 3_600_000, '3 hours ago'],
    [86_400_000, 'yesterday'],
    [3 * 86_400_000, '3 days ago'],
    [14 * 86_400_000, '2 weeks ago'],
    [90 * 86_400_000, '3 months ago'],
    [730 * 86_400_000, '2 years ago'],
  ])('formats %i milliseconds naturally', (elapsed, label) => {
    expect(lastRideLabel(now - elapsed, now)).toBe(`Last ride was ${label}`);
  });
  it('handles empty history and clock skew', () => {
    expect(lastRideLabel(null, now)).toBe('No rides recorded yet');
    expect(lastRideLabel(now + 60_000, now)).toBe('Last ride was just now');
    expect(latestRideAt([])).toBeNull();
    expect(latestRideAt([board('invalid'), board('2026-01-01T00:00:00Z')])).toBe(Date.parse('2026-01-01T00:00:00Z'));
  });
});
