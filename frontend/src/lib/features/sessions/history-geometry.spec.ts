import { describe, expect, it } from 'vitest';
import { axisTicks, buildPathSegments, elapsedTicks, finiteDomain, formatAxisTick, zeroReference } from './history-geometry';

describe('historical session geometry', () => {
  it('pads constant finite domains deterministically and exposes zero when applicable', () => {
    const domain = finiteDomain([0, 0, null]);
    expect(domain).toEqual({ min: -1, max: 1 });
    expect(zeroReference(domain)).toBe(0.5);
  });

  it('breaks paths at nulls and continuity generations', () => {
    const segments = buildPathSegments(
      {
        elapsedMs: [0, 1, 2, 3],
        sequence: [0, 1, 2, 3],
        values: [1, 2, null, 3],
        generations: [0, 0, 1, 1],
      },
      { width: 100, height: 50, viewport: { startElapsedMs: 0, endElapsedMs: 3 } }
    );
    expect(segments).toHaveLength(2);
    expect(segments[0]).toContain('M');
    expect(segments[1]).toMatch(/^M 100\.00 /);
  });

  it('keeps shared elapsed ticks inside the visible range', () => {
    expect(elapsedTicks({ startElapsedMs: 10, endElapsedMs: 20 }, 3)).toEqual([10, 15, 20]);
  });

  it('generates evenly spaced numeric axis ticks and compact labels', () => {
    expect(axisTicks({ min: -1, max: 3 }, 5)).toEqual([-1, 0, 1, 2, 3]);
    expect(formatAxisTick(62.125)).toBe('62.1');
    expect(formatAxisTick(0.52125)).toBe('0.521');
  });
});
