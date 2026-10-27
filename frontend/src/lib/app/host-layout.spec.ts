import { describe, expect, it } from 'vitest';
import { closedSideLayout } from '../../../../conformance/test/support/closed-side-layout';
import { readHostLayout, sideControls } from './host-layout';

const viewport = { width: 466, height: 678 };
const layout = () => structuredClone(closedSideLayout);

describe('host layout facts', () => {
  it('copies only validated geometry so later mutations cannot bypass validation', () => {
    const input = layout();
    const parsed = readHostLayout(input);
    expect(parsed).toEqual(input);
    input.occlusions[0].height = 10000;
    expect(parsed?.occlusions[0].height).toBe(37);
  });

  it.each([
    null,
    {},
    { ...closedSideLayout, version: 2 },
    { ...closedSideLayout, edge: 'trailing' },
    { ...closedSideLayout, bounds: { width: NaN, height: 678 } },
    { ...closedSideLayout, safeArea: { top: 0, right: -1, bottom: 34, left: 0 } },
    { ...closedSideLayout, safeArea: { top: 0, right: 500, bottom: 34, left: 0 } },
    { ...closedSideLayout, occlusions: Array(17).fill({ x: 0, y: 0, width: 1, height: 1 }) },
    { ...closedSideLayout, occlusions: [{ x: 0, y: 0, width: 1, height: Infinity }] },
  ])('rejects unsupported or malformed geometry: %j', (value) => expect(readHostLayout(value)).toBeNull());
});

describe('side control placement', () => {
  it('uses the actual clear interval below camera/status regions, not StatusBarManager height', () => {
    expect(sideControls(readHostLayout(closedSideLayout), viewport)).toEqual({
      edge: 'right',
      x: 386,
      y: 182,
      width: 76,
      height: 454,
      space: 84,
    });
  });

  it('recomputes when a Live Activity extends the occlusion', () => {
    const value = layout();
    value.occlusions[1].height = 250;
    expect(sideControls(value, viewport)?.y).toBe(258);
    expect(sideControls(value, viewport)?.height).toBe(378);
    value.occlusions[1].height = 400;
    expect(sideControls(value, viewport)).toBeNull();
  });

  it('subtracts middle obstructions and chooses the largest remaining interval', () => {
    const value = layout();
    value.occlusions.push({ x: 382, y: 550, width: 84, height: 40 });
    expect(sideControls(value, viewport)?.height).toBe(360);
  });

  it('falls back when overlapping system regions cover the entire side column', () => {
    const value = layout();
    value.occlusions = Array.from({ length: 16 }, () => ({ x: 382, y: 0, width: 84, height: 678 }));
    expect(sideControls(value, viewport)).toBeNull();
  });

  it('handles a physical left-side region without mirroring product navigation', () => {
    const value = {
      ...layout(),
      edge: 'left' as const,
      safeArea: { top: 0, right: 0, bottom: 34, left: 84 },
      occlusions: [{ x: 0, y: 0, width: 84, height: 174 }],
    };
    expect(sideControls(value, viewport)).toEqual({ edge: 'left', x: 4, y: 182, width: 76, height: 454, space: 84 });
  });

  it('falls back for stale resize data, zoom, large text, narrow columns and absent edges', () => {
    expect(sideControls(null, viewport)).toBeNull();
    expect(sideControls({ ...layout(), edge: null }, viewport)).toBeNull();
    expect(sideControls(layout(), { ...viewport, width: 678 })).toBeNull();
    expect(sideControls(layout(), { ...viewport, scale: 2 })).toBeNull();
    expect(sideControls(layout(), { ...viewport, fontSize: 32 })).toBeNull();
    expect(sideControls({ ...layout(), safeArea: { ...layout().safeArea, right: 44 } }, viewport)).toBeNull();
  });
});
