import { describe, expect, it } from 'vitest';
import { chartTouchInspection } from './chart-touch-inspection';

function setup(enabled = true) {
  const node = new EventTarget();
  const action = chartTouchInspection(node as SVGElement, enabled);
  function touch(type: string, x = 100, y = 100, count = 1, cancelable = true) {
    const event = new Event(type, { cancelable });
    Object.defineProperty(event, 'touches', {
      value: Array.from({ length: count }, (_, identifier) => ({ identifier, clientX: x, clientY: y })),
    });
    node.dispatchEvent(event);
    return event.defaultPrevented;
  }
  return { action, touch };
}

describe('chart touch inspection scrolling', () => {
  it('locks horizontal inspection through later vertical drift until release', () => {
    const { touch } = setup();
    touch('touchstart');
    expect(touch('touchmove', 102, 101)).toBe(false);
    expect(touch('touchmove', 105, 101)).toBe(true);
    expect(touch('touchmove', 105, 160)).toBe(true);
    touch('touchend', 0, 0, 0);
    touch('touchstart');
    expect(touch('touchmove', 101, 120)).toBe(false);
  });

  it('keeps a vertical start scrollable even after horizontal movement', () => {
    const { touch } = setup();
    touch('touchstart');
    expect(touch('touchmove', 101, 105)).toBe(false);
    expect(touch('touchmove', 180, 105)).toBe(false);
  });

  it('releases cancellation and ignores other modes and destroyed actions', () => {
    const { action, touch } = setup();
    touch('touchstart');
    expect(touch('touchmove', 110)).toBe(true);
    touch('touchcancel', 0, 0, 0);
    expect(touch('touchmove', 120)).toBe(false);
    action.update(false);
    touch('touchstart');
    expect(touch('touchmove', 120)).toBe(false);
    action.update(true);
    action.destroy();
    touch('touchstart');
    expect(touch('touchmove', 120)).toBe(false);
  });

  it('does not claim multi-touch starts or cancel noncancelable events', () => {
    const { touch } = setup();
    touch('touchstart', 100, 100, 2);
    expect(touch('touchmove', 120, 100, 2)).toBe(false);
    touch('touchend', 0, 0, 0);
    touch('touchstart');
    expect(touch('touchmove', 120, 100, 1, false)).toBe(false);
  });
});
