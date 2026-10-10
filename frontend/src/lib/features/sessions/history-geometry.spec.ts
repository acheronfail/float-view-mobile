import { describe, expect, it } from 'vitest';
import {
  axisTicks,
  buildPathSegments,
  elapsedTicks,
  finiteDomain,
  formatAxisTick,
  pointAtElapsed,
  zeroReference,
} from './history-geometry';
import { sessionChartGroups } from './session-fields';

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

  it('retains the first sample of each generation without bridging gaps', () => {
    const segments = buildPathSegments(
      { elapsedMs: [0, 1, 2, 3], sequence: [1, 2, 3, 4], values: [1, 1, 1, 1], generations: [0, 0, 1, 1] },
      { width: 90, height: 50, viewport: { startElapsedMs: 0, endElapsedMs: 3 }, domain: { min: 0, max: 2 } }
    );
    expect(segments).toEqual(['M 0.00 25.00 L 30.00 25.00', 'M 60.00 25.00 L 90.00 25.00']);
  });

  it('keeps isolated readings drawable when every sample follows a gap', () => {
    const segments = buildPathSegments(
      { elapsedMs: [0, 1500, 3000], sequence: [1, 2, 3], values: [1, 1, 1], generations: [0, 1, 2] },
      { width: 100, height: 50, viewport: { startElapsedMs: 0, endElapsedMs: 3000 }, domain: { min: 0, max: 2 } }
    );
    expect(segments).toEqual(['M 0.00 25.00 l 0 0', 'M 50.00 25.00 l 0 0', 'M 100.00 25.00 l 0 0']);
  });

  it('finds the interpolated point where the inspection bar crosses a line', () => {
    expect(
      pointAtElapsed(
        {
          elapsedMs: [0, 10],
          sequence: [0, 1],
          values: [0, 10],
          generations: [0, 0],
        },
        5,
        { min: -1, max: 11 },
        { width: 100, height: 50, viewport: { startElapsedMs: 0, endElapsedMs: 10 } }
      )
    ).toEqual({ elapsedMs: 5, value: 5, x: 50, y: 25 });
  });

  it('does not cross null or generation gaps', () => {
    const column = {
      elapsedMs: [0, 10, 20],
      sequence: [0, 1, 2],
      values: [0, null, 10],
      generations: [0, 1, 1],
    };
    expect(
      pointAtElapsed(
        column,
        5,
        { min: -1, max: 11 },
        { width: 100, height: 50, viewport: { startElapsedMs: 0, endElapsedMs: 20 } }
      )
    ).toBeNull();
    expect(
      pointAtElapsed(
        column,
        15,
        { min: -1, max: 11 },
        { width: 100, height: 50, viewport: { startElapsedMs: 0, endElapsedMs: 20 } }
      )
    ).toBeNull();
  });

  it('keeps shared elapsed ticks inside the visible range', () => {
    expect(elapsedTicks({ startElapsedMs: 10, endElapsedMs: 20 }, 3)).toEqual([10, 15, 20]);
  });

  it('generates evenly spaced numeric axis ticks and compact labels', () => {
    expect(axisTicks({ min: -1, max: 3 }, 5)).toEqual([-1, 0, 1, 2, 3]);
    expect(formatAxisTick(62.125)).toBe('62.1');
    expect(formatAxisTick(0.52125)).toBe('0.521');
  });

  it('uses a fixed domain without padding the visible values', () => {
    const duty = sessionChartGroups([{ name: 'dutyCycle' }])[0]!;
    expect(finiteDomain([0.25, 0.5, 0.75], duty.axis)).toEqual({ min: 0, max: 1 });
    expect(axisTicks(finiteDomain([0.25, 0.5, 0.75], duty.axis), 3)).toEqual([0, 0.5, 1]);
  });

  it('shares one automatic domain across the temperature fields', () => {
    const temperature = sessionChartGroups([{ name: 'motorTempC' }, { name: 'controllerTempC' }])[0]!;
    const domain = finiteDomain([28, 86, 72, 30], temperature.axis);
    expect(domain).toEqual({ min: 25.1, max: 88.9 });
    expect(
      buildPathSegments(
        {
          elapsedMs: [0, 1],
          sequence: [0, 1],
          values: [28, 30],
          generations: [0, 0],
        },
        { width: 100, height: 50, viewport: { startElapsedMs: 0, endElapsedMs: 1 }, domain: domain! }
      )
    ).toHaveLength(1);
  });

  it('places inspection dots for every line at the shared elapsed position', () => {
    const domain = { min: 20, max: 100 };
    const motor = pointAtElapsed(
      { elapsedMs: [0, 10], sequence: [0, 1], values: [28, 32], generations: [0, 0] },
      5,
      domain,
      { width: 320, height: 96, viewport: { startElapsedMs: 0, endElapsedMs: 10 } }
    );
    const controller = pointAtElapsed(
      { elapsedMs: [0, 10], sequence: [0, 1], values: [30, 40], generations: [0, 0] },
      5,
      domain,
      { width: 320, height: 96, viewport: { startElapsedMs: 0, endElapsedMs: 10 } }
    );

    expect(motor?.x).toBe(160);
    expect(controller?.x).toBe(160);
    expect(motor?.y).not.toBe(controller?.y);
  });
});
