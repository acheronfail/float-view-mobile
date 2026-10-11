import { afterEach, describe, expect, it, vi } from 'vitest';
import { startRideRefresh } from './ride-refresh';

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe('current ride refresh lifecycle', () => {
  it('bounds in-flight work and rejects responses after hide, inspection and disposal', async () => {
    vi.useFakeTimers();
    const document = Object.assign(new EventTarget(), { hidden: false });
    vi.stubGlobal('document', document);
    let following = true;
    let accept!: () => boolean;
    let finish!: () => void;
    const refresh = vi.fn((allowed: () => boolean) => {
      accept = allowed;
      return new Promise<void>((resolve) => {
        finish = resolve;
      });
    });
    const runner = startRideRefresh({ shouldRefresh: () => following, refresh, onerror: vi.fn() });
    await vi.advanceTimersByTimeAsync(9000);
    expect(refresh).toHaveBeenCalledTimes(1);
    expect(accept()).toBe(true);
    document.hidden = true;
    document.dispatchEvent(new Event('visibilitychange'));
    expect(accept()).toBe(false);
    document.hidden = false;
    document.dispatchEvent(new Event('visibilitychange'));
    expect(accept()).toBe(false);
    finish();
    await vi.advanceTimersByTimeAsync(3000);
    expect(refresh).toHaveBeenCalledTimes(2);
    following = false;
    expect(accept()).toBe(false);
    finish();
    await vi.advanceTimersByTimeAsync(6000);
    expect(refresh).toHaveBeenCalledTimes(2);
    following = true;
    void runner.refresh();
    expect(refresh).toHaveBeenCalledTimes(3);
    runner.dispose();
    expect(accept()).toBe(false);
    finish();
    await vi.advanceTimersByTimeAsync(6000);
    expect(refresh).toHaveBeenCalledTimes(3);
  });

  it('reports failures and retries without overlapping', async () => {
    vi.useFakeTimers();
    vi.stubGlobal('document', Object.assign(new EventTarget(), { hidden: false }));
    const onerror = vi.fn();
    const refresh = vi.fn().mockRejectedValueOnce(new Error('offline')).mockResolvedValue(undefined);
    const runner = startRideRefresh({ shouldRefresh: () => true, refresh, onerror });
    await vi.advanceTimersByTimeAsync(6000);
    expect(refresh).toHaveBeenCalledTimes(2);
    expect(onerror).toHaveBeenCalledTimes(1);
    runner.dispose();
  });
});
