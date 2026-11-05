import { afterEach, describe, expect, it, vi } from 'vitest';
import { publishHostBackNavigation } from './host-back-navigation';
import { installBackNavigation } from '$lib/ui/back-dismissal';

afterEach(() => vi.unstubAllGlobals());

describe('host back navigation', () => {
  it('publishes only a boolean presentation capability', () => {
    const postMessage = vi.fn();
    vi.stubGlobal('webkit', { messageHandlers: { floatViewBackNavigation: { postMessage } } });
    publishHostBackNavigation(false);
    publishHostBackNavigation(true);
    expect(postMessage.mock.calls).toEqual([[false], [true]]);
  });

  it('preserves ordinary navigation when no host exists or its bridge fails', () => {
    vi.stubGlobal('webkit', undefined);
    expect(() => publishHostBackNavigation(true)).not.toThrow();
    vi.stubGlobal('webkit', {
      messageHandlers: {
        floatViewBackNavigation: {
          postMessage: () => {
            throw new Error('Unavailable');
          },
        },
      },
    });
    expect(() => publishHostBackNavigation(true)).not.toThrow();
  });

  it('rechecks edge eligibility without changing system Back behavior', () => {
    const target = new EventTarget();
    vi.stubGlobal('window', target);
    const back = vi.fn(() => true);
    let eligible = false;
    const stop = installBackNavigation(back, () => eligible);
    const edge = () => new CustomEvent('floatview:back', { cancelable: true, detail: { source: 'edge' } });
    expect(target.dispatchEvent(edge())).toBe(true);
    expect(back).not.toHaveBeenCalled();
    expect(target.dispatchEvent(new Event('floatview:back', { cancelable: true }))).toBe(false);
    expect(back).toHaveBeenCalledTimes(1);
    expect(back).toHaveBeenLastCalledWith('system', false);
    eligible = true;
    expect(target.dispatchEvent(edge())).toBe(false);
    expect(back).toHaveBeenCalledTimes(2);
    expect(back).toHaveBeenLastCalledWith('edge', true);
    stop();
    target.dispatchEvent(edge());
    expect(back).toHaveBeenCalledTimes(2);
  });

  it('acknowledges interactive Back only after asynchronous presentation is ready', async () => {
    const target = new EventTarget();
    vi.stubGlobal('window', target);
    let ready!: (accepted: boolean) => void;
    const back = () => new Promise<boolean>((resolve) => (ready = resolve));
    const stop = installBackNavigation(back, () => true);
    const complete = vi.fn();
    expect(
      target.dispatchEvent(
        new CustomEvent('floatview:back', {
          cancelable: true,
          detail: { source: 'edge', complete },
        })
      )
    ).toBe(false);
    expect(complete).not.toHaveBeenCalled();
    ready(true);
    await Promise.resolve();
    expect(complete).toHaveBeenCalledExactlyOnceWith(true);
    stop();
  });

  it('keeps edge eligibility while allowing ordinary motion when capture is unavailable', async () => {
    const target = new EventTarget();
    vi.stubGlobal('window', target);
    const back = vi.fn(() => true);
    const complete = vi.fn();
    const stop = installBackNavigation(back, () => true);
    target.dispatchEvent(
      new CustomEvent('floatview:back', {
        cancelable: true,
        detail: { source: 'edge', interactive: false, complete },
      })
    );
    await Promise.resolve();
    expect(back).toHaveBeenCalledExactlyOnceWith('edge', false);
    expect(complete).toHaveBeenCalledExactlyOnceWith(true);
    stop();
  });

  it.each([false, 'failure', 'ineligible'])('settles back for %s navigation', async (outcome) => {
    const target = new EventTarget();
    vi.stubGlobal('window', target);
    const back = () => (outcome === 'failure' ? Promise.reject(new Error('Failed')) : false);
    const stop = installBackNavigation(back, () => outcome !== 'ineligible');
    const complete = vi.fn();
    target.dispatchEvent(new CustomEvent('floatview:back', { cancelable: true, detail: { source: 'edge', complete } }));
    await Promise.resolve();
    expect(complete).toHaveBeenCalledExactlyOnceWith(false);
    stop();
  });
});
