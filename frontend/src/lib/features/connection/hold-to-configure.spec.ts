import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { BOX_HOLD_MS, holdToConfigure } from './hold-to-configure';
function pointer(node: EventTarget, type: string, values = {}) {
  node.dispatchEvent(
    Object.assign(new Event(type), { pointerId: 1, button: 0, isPrimary: true, clientX: 10, clientY: 10, ...values })
  );
}
beforeEach(() => {
  vi.useFakeTimers();
  vi.stubGlobal('window', new EventTarget());
  vi.stubGlobal('document', new EventTarget());
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});
function setup() {
  const node = new EventTarget() as HTMLButtonElement;
  const onhold = vi.fn(),
    ontap = vi.fn();
  const action = holdToConfigure(node, { key: 'leds:board', onhold, ontap });
  return { node, onhold, ontap, action };
}
describe('live box hold gesture', () => {
  it('separates short taps from holds and ignores synthetic clicks', () => {
    const { node, onhold, ontap, action } = setup();
    pointer(node, 'pointerdown');
    vi.advanceTimersByTime(BOX_HOLD_MS - 1);
    pointer(node, 'pointerup');
    expect(ontap).toHaveBeenCalledTimes(1);
    expect(onhold).not.toHaveBeenCalled();
    pointer(node, 'pointerdown');
    vi.advanceTimersByTime(BOX_HOLD_MS);
    expect(onhold).toHaveBeenCalledTimes(1);
    pointer(node, 'pointerup');
    node.dispatchEvent(new Event('click'));
    vi.advanceTimersByTime(2000);
    expect(ontap).toHaveBeenCalledTimes(1);
    expect(onhold).toHaveBeenCalledTimes(1);
    action.destroy();
  });
  it.each(['pointercancel', 'lostpointercapture', 'movement', 'blur', 'hidden', 'destroy', 'replacement'])(
    'cancels on %s',
    (reason) => {
      const { node, onhold, ontap, action } = setup();
      pointer(node, 'pointerdown');
      if (reason === 'movement') pointer(node, 'pointermove', { clientY: 40 });
      else if (reason === 'blur') window.dispatchEvent(new Event('blur'));
      else if (reason === 'hidden') document.dispatchEvent(new Event('visibilitychange'));
      else if (reason === 'destroy') action.destroy();
      else if (reason === 'replacement') action.update({ key: 'empty:board', onhold, ontap });
      else pointer(node, reason);
      vi.advanceTimersByTime(BOX_HOLD_MS);
      pointer(node, 'pointerup');
      expect(onhold).not.toHaveBeenCalled();
      expect(ontap).not.toHaveBeenCalled();
      action.destroy();
    }
  );
  it('ignores non-primary and secondary buttons and retains holds across telemetry updates', () => {
    const { node, onhold, ontap, action } = setup();
    for (const values of [{ isPrimary: false }, { button: 2 }]) {
      pointer(node, 'pointerdown', values);
      vi.advanceTimersByTime(BOX_HOLD_MS);
      pointer(node, 'pointerup', values);
    }
    expect(onhold).not.toHaveBeenCalled();
    expect(ontap).not.toHaveBeenCalled();
    pointer(node, 'pointerdown');
    vi.advanceTimersByTime(300);
    action.update({ key: 'leds:board', onhold, ontap });
    vi.advanceTimersByTime(300);
    expect(onhold).toHaveBeenCalledTimes(1);
    action.destroy();
  });
});
