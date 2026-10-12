import { describe, expect, it, vi } from 'vitest';
import {
  activateFromKeyboardOrAssistiveClick,
  activateFromPointer,
  isKeyboardOrAssistiveClick,
  isPrimaryPointerActivation,
} from './activation';

describe('cross-platform control activation', () => {
  it('accepts only the primary pointer', () => {
    expect(isPrimaryPointerActivation({ button: 0, isPrimary: true })).toBe(true);
    expect(isPrimaryPointerActivation({ button: 1, isPrimary: true })).toBe(false);
    expect(isPrimaryPointerActivation({ button: 0, isPrimary: false })).toBe(false);
  });

  it('keeps synthesized keyboard and assistive clicks without repeating pointer clicks', () => {
    expect(isKeyboardOrAssistiveClick({ detail: 0 })).toBe(true);
    expect(isKeyboardOrAssistiveClick({ detail: 1 })).toBe(false);
  });

  it('dispatches each supported activation path once after pointer release', () => {
    const action = vi.fn();
    const preventDefault = vi.fn();
    const target = new EventTarget();
    const pointerup = new Event('pointerup');
    Object.defineProperties(pointerup, {
      pointerId: { value: 7 },
      isPrimary: { value: true },
    });

    activateFromPointer(
      {
        button: 0,
        isPrimary: true,
        pointerId: 7,
        currentTarget: target,
        preventDefault,
      } as unknown as PointerEvent,
      action
    );
    expect(action).not.toHaveBeenCalled();
    target.dispatchEvent(pointerup);
    activateFromKeyboardOrAssistiveClick({ detail: 1 } as MouseEvent, action);
    activateFromKeyboardOrAssistiveClick({ detail: 0 } as MouseEvent, action);

    expect(preventDefault).toHaveBeenCalledOnce();
    expect(action).toHaveBeenCalledTimes(2);
  });

  it('does not activate after pointer cancellation', () => {
    const action = vi.fn();
    const target = new EventTarget();
    const event = (type: string) => {
      const next = new Event(type);
      Object.defineProperty(next, 'pointerId', { value: 9 });
      return next;
    };
    activateFromPointer(
      {
        button: 0,
        isPrimary: true,
        pointerId: 9,
        currentTarget: target,
        preventDefault() {},
      } as unknown as PointerEvent,
      action
    );
    target.dispatchEvent(event('pointercancel'));
    target.dispatchEvent(event('pointerup'));
    expect(action).not.toHaveBeenCalled();
  });

  it.each(['drag', 'release', 'cancel', 'tap'])(
    'scrollable choices handle %s without suppressing scrolling',
    (gesture) => {
      const action = vi.fn();
      const preventDefault = vi.fn();
      const target = new EventTarget();
      const send = (type: string, y: number, pointerId = 9) => {
        const next = new Event(type);
        Object.assign(next, { pointerId, clientX: 20, clientY: y, isPrimary: true });
        target.dispatchEvent(next);
      };
      activateFromPointer(
        {
          button: 0,
          isPrimary: true,
          pointerId: 9,
          clientX: 20,
          clientY: 20,
          currentTarget: target,
          preventDefault,
        } as unknown as PointerEvent,
        action,
        { scrollable: true }
      );
      // An unrelated finger cannot cancel the primary tap.
      send('pointermove', 100, 10);
      if (gesture === 'drag') {
        send('pointermove', 60);
        send('pointermove', 20);
      }
      if (gesture === 'cancel') send('pointercancel', 20);
      send('pointerup', gesture === 'release' ? 60 : 20);
      activateFromKeyboardOrAssistiveClick({ detail: 1 } as MouseEvent, action);
      expect(preventDefault).not.toHaveBeenCalled();
      expect(action).toHaveBeenCalledTimes(gesture === 'tap' ? 1 : 0);
    }
  );
});
