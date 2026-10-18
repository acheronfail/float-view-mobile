import { describe, expect, it } from 'vitest';
import { render } from 'svelte/server';
import { defaultBoardAppearance, boardAppearanceColours } from '$lib/backend/contract';
import { validBoardAppearance } from '$lib/backend/board-appearance';
import { appearanceShapes, cycleAppearance } from './appearance';
import BoardIcon from './BoardIcon.svelte';

describe('board appearance', () => {
  it('defaults to style one and neutral board colours', () => {
    expect(defaultBoardAppearance).toEqual({
      rail: 1,
      tyre: 1,
      fender: 1,
      hooks: 1,
      leftTop: '#a9afb8',
      leftBottom: '#a9afb8',
      rightTop: '#a9afb8',
      rightBottom: '#a9afb8',
      rubber: '#59616b',
      rim: '#17191d',
      fenders: '#a9afb8',
      hookColour: '#ffffff',
    });
    expect(validBoardAppearance(defaultBoardAppearance)).toBe(true);
  });
  it('keeps hook/fender combinations valid and cycles at both ends', () => {
    expect(cycleAppearance({ ...defaultBoardAppearance }, 'rail', -1).rail).toBe(6);
    expect(cycleAppearance({ ...defaultBoardAppearance, rail: 6 }, 'rail', 1).rail).toBe(1);
    const hooked = cycleAppearance({ ...defaultBoardAppearance }, 'hooks', 1);
    expect(hooked).toMatchObject({ hooks: 2, fender: 2 });
    expect(cycleAppearance(hooked, 'fender', -1)).toMatchObject({ hooks: 1, fender: 1 });
    expect(cycleAppearance({ ...defaultBoardAppearance, hooks: 3 }, 'fender', 1).hooks).toBe(3);
  });
  it('rejects unsupported or incomplete appearance data', () => {
    for (const patch of [{ rail: 0 }, { rail: 7 }, { tyre: 1.5 }, { hooks: 2 }, { leftTop: 'red' }, { extra: 1 }])
      expect(validBoardAppearance({ ...defaultBoardAppearance, ...patch })).toBe(false);
    expect(validBoardAppearance({ rail: 1 })).toBe(false);
    for (const [, colour] of boardAppearanceColours)
      expect(validBoardAppearance({ ...defaultBoardAppearance, rim: colour })).toBe(true);
  });
  it('renders both wheels, every rail, and hooks without shared SVG identifiers', () => {
    for (const rail of [1, 2, 3, 4, 5, 6] as const)
      for (const tyre of [1, 2] as const) {
        const value = { ...defaultBoardAppearance, rail, tyre, hooks: 3 as const };
        const shapes = appearanceShapes(value);
        expect(shapes.tyre).not.toMatch(/NaN|undefined/);
        expect(shapes.hooks).toHaveLength(2);
        const body = render(BoardIcon, { props: { appearance: value } }).body;
        expect(body).toContain(`points="${shapes.rail}"`);
        expect(body).toContain('r="4.2"');
        expect(body).not.toContain('NaN');
      }
  });
});
