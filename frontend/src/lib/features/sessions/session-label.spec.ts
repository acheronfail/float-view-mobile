import { describe, expect, it } from 'vitest';
import { sessionStartDateLabel } from './session-label';

describe('session labels', () => {
  it('formats a session title from its start date', () => {
    expect(sessionStartDateLabel('2026-08-05T01:02:03.000Z')).toBe(
      new Intl.DateTimeFormat(undefined, { year: 'numeric', month: 'long', day: 'numeric' }).format(
        new Date('2026-08-05T01:02:03.000Z')
      )
    );
  });

  it('returns no title for an invalid start date', () => {
    expect(sessionStartDateLabel('not-a-date')).toBeNull();
  });
});
