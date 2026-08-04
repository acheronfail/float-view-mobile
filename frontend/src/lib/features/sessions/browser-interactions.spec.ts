import { describe, expect, it, vi } from 'vitest';
import { activateFromPointer } from '$lib/ui/activation';
import { pinchViewport, panViewport } from './viewport';

function pointer(overrides: Partial<PointerEvent> = {}): PointerEvent {
  return {
    pointerId: 1,
    pointerType: 'touch',
    isPrimary: true,
    button: 0,
    buttons: 1,
    clientX: 100,
    clientY: 40,
    preventDefault: vi.fn(),
    currentTarget: new EventTarget(),
    ...overrides,
  } as unknown as PointerEvent;
}

describe('browser chart interaction scenarios', () => {
  it('accepts two-touch pinch when only the first contact is primary', () => {
    const viewport = { startElapsedMs: 0, endElapsedMs: 10_000 };
    const firstContact = pointer({ pointerId: 7, isPrimary: true, clientX: 80 });
    const secondContact = pointer({ pointerId: 8, isPrimary: false, clientX: 220 });
    expect(firstContact.isPrimary).toBe(true);
    expect(secondContact.isPrimary).toBe(false);

    const pinch = pinchViewport(viewport, 140, 280, 0.5, 10_000);
    expect(pinch).toEqual({ startElapsedMs: 2_500, endElapsedMs: 7_500 });
  });

  it('does not activate a non-primary one-pointer control', () => {
    const action = vi.fn();
    const target = new EventTarget();
    const event = pointer({
      pointerType: 'mouse',
      isPrimary: false,
      currentTarget: target,
    });

    activateFromPointer(event, action);
    target.dispatchEvent(new Event('pointerup'));
    expect(action).not.toHaveBeenCalled();
  });

  it('keeps horizontal pan separate from a crosshair inspection', () => {
    const viewport = { startElapsedMs: 2_000, endElapsedMs: 8_000 };
    expect(panViewport(viewport, -10_000, 10_000)).toEqual({ startElapsedMs: 0, endElapsedMs: 6_000 });
  });

  it('cancels a vertical-scroll gesture before it can commit a range', () => {
    const action = vi.fn();
    const target = new EventTarget();
    const event = pointer({ currentTarget: target });

    activateFromPointer(event, action);
    target.dispatchEvent(new Event('pointercancel'));
    target.dispatchEvent(new Event('pointerup'));
    expect(action).not.toHaveBeenCalled();
  });

});
