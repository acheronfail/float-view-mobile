import { describe, expect, it } from 'vitest';
import { shouldDismissSheet } from './sheet';

describe('bottom-sheet release', () => {
  it('dismisses after a deliberate pull of about six percent', () => {
    expect(shouldDismissSheet({ distancePx: 47, heightPx: 800, velocityPxPerMs: 0.1 })).toBe(false);
    expect(shouldDismissSheet({ distancePx: 48, heightPx: 800, velocityPxPerMs: 0.1 })).toBe(true);
  });

  it('dismisses a short, fast downward fling', () => {
    expect(shouldDismissSheet({ distancePx: 12, heightPx: 800, velocityPxPerMs: 0.7 })).toBe(true);
  });

  it('snaps back after a short or upward release', () => {
    expect(shouldDismissSheet({ distancePx: 20, heightPx: 800, velocityPxPerMs: 0.2 })).toBe(false);
    expect(shouldDismissSheet({ distancePx: 12, heightPx: 800, velocityPxPerMs: -0.7 })).toBe(false);
  });
});
