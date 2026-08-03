import { describe, expect, it, vi } from 'vitest';
import { activateFromPointer } from '$lib/ui/activation';
import { pinchViewport, panViewport } from './viewport';
import {
  ExactTooltipController,
  formatElapsedForAssistive,
  tooltipPlacement,
  type ExactTooltipValue,
} from './exact-tooltip';

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((nextResolve, nextReject) => {
    resolve = nextResolve;
    reject = nextReject;
  });
  return { promise, resolve, reject };
}

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
  it('makes mouse hover and touch/pen inspection share exact value semantics', async () => {
    const exact: ExactTooltipValue = { elapsedMs: 1_004, values: { speedKph: 18.5 } };
    const lookup = vi.fn(async () => exact);
    const controller = new ExactTooltipController(lookup);

    controller.inspect(1_000, 'speedKph', false);
    await vi.waitFor(() => expect(controller.snapshot.status).toBe('exact'));
    expect(controller.snapshot.value).toEqual(exact);
    expect(controller.snapshot.pinned).toBe(false);

    controller.inspect(1_000, 'speedKph', true);
    await vi.waitFor(() => expect(lookup).toHaveBeenCalledTimes(2));
    await vi.waitFor(() => expect(controller.snapshot.status).toBe('exact'));
    expect(controller.snapshot.value).toEqual(exact);
    expect(controller.snapshot.pinned).toBe(true);
  });

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

  it('coalesces rapid hover targets and suppresses the older response', async () => {
    const first = deferred<ExactTooltipValue>();
    const second = deferred<ExactTooltipValue>();
    const lookup = vi.fn().mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    const controller = new ExactTooltipController(lookup);

    controller.inspect(100, 'speedKph', false);
    controller.inspect(200, 'speedKph', false);
    expect(controller.snapshot).toMatchObject({ status: 'pending', targetElapsedMs: 200 });
    expect(lookup).toHaveBeenCalledTimes(1);

    first.resolve({ elapsedMs: 100, values: { speedKph: 10 } });
    await vi.waitFor(() => expect(lookup).toHaveBeenCalledTimes(2));
    expect(controller.snapshot.status).toBe('pending');
    second.resolve({ elapsedMs: 200, values: { speedKph: 20 } });
    await vi.waitFor(() => expect(controller.snapshot.status).toBe('exact'));
    expect(controller.snapshot.value?.values.speedKph).toBe(20);
  });

  it('reports unavailable values instead of substituting a neighboring value', async () => {
    const controller = new ExactTooltipController(async () => ({ elapsedMs: 500, values: { speedKph: null } }));
    controller.inspect(500, 'speedKph', true);
    await vi.waitFor(() => expect(controller.snapshot.status).toBe('unavailable'));
    expect(controller.snapshot.value?.values.speedKph).toBeNull();
  });

  it('dismisses hover, preserves edge-safe placement, and exposes assistive value text', async () => {
    const controller = new ExactTooltipController(async () => ({ elapsedMs: 0, values: { speedKph: 1 } }));
    controller.inspect(0, 'speedKph', false);
    await vi.waitFor(() => expect(controller.snapshot.status).toBe('exact'));
    controller.dismiss();
    expect(controller.snapshot.status).toBe('idle');
    expect(tooltipPlacement(318, 120, 320)).toEqual({ leftPx: 194, side: 'before' });
    expect(tooltipPlacement(2, 120, 320)).toEqual({ leftPx: 6, side: 'after' });
    expect(formatElapsedForAssistive(62_000, 125_000)).toBe('Elapsed 1m 2s of 2m 5s');
  });

  it('ignores a late response after the route has been disposed', async () => {
    const pending = deferred<ExactTooltipValue>();
    const onChange = vi.fn();
    const controller = new ExactTooltipController(() => pending.promise, onChange);
    controller.inspect(900, 'speedKph', true);
    controller.dispose();
    pending.resolve({ elapsedMs: 900, values: { speedKph: 9 } });
    await Promise.resolve();
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(controller.snapshot.status).toBe('pending');
  });
});
