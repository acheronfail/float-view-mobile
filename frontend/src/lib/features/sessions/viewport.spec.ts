import { describe, expect, it } from 'vitest';
import { clampViewport, panViewport, pinchViewport } from './viewport';

describe('session viewport math', () => {
  it('clamps a viewport to a one-millisecond minimum and duration', () => {
    expect(clampViewport({ startElapsedMs: 0, endElapsedMs: 0 }, 1_000)).toEqual({
      startElapsedMs: 0,
      endElapsedMs: 1,
    });
    expect(clampViewport({ startElapsedMs: 900, endElapsedMs: 2_000 }, 1_000)).toEqual({
      startElapsedMs: 0,
      endElapsedMs: 1_000,
    });
  });

  it('pans without leaving the full-duration bounds', () => {
    expect(panViewport({ startElapsedMs: 200, endElapsedMs: 600 }, -500, 1_000)).toEqual({
      startElapsedMs: 0,
      endElapsedMs: 400,
    });
  });

  it('preserves the pinch midpoint while zooming', () => {
    expect(pinchViewport({ startElapsedMs: 0, endElapsedMs: 1_000 }, 100, 200, 0.5, 1_000)).toEqual({
      startElapsedMs: 250,
      endElapsedMs: 750,
    });
  });
});
