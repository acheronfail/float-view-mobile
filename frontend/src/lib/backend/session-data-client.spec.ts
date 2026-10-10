import { afterEach, describe, expect, it, vi } from 'vitest';
import { SessionDataClient } from './session-data-client';
import type { SessionDataRequest } from './session-data';

const request: SessionDataRequest = {
  kind: 'events',
  sessionId: 'test-session',
  snapshot: {
    historyRevision: 1,
    committedSampleSequence: 1,
    committedEventSequence: 0,
    committedElapsedMs: 100,
    committedLocationSequence: 0,
    committedLocationElapsedMs: 0,
  },
  range: { startElapsedMs: 0, endElapsedMs: 100 },
  limit: 100,
  cursor: null,
};
function success() {
  return new Response(
    JSON.stringify({
      ...request,
      page: { elapsedMs: [], sequence: [], sourceSampleSequence: [], kinds: [], values: [] },
      nextCursor: null,
    }),
    { headers: { 'Content-Type': 'application/json' } }
  );
}
function client(fetcher: typeof fetch) {
  return new SessionDataClient({ webSocketUrl: '/api/ws', sessionToken: 'token' }, false, 'http://127.0.0.1/', fetcher);
}
afterEach(() => vi.useRealTimers());

describe('session-data scheduling', () => {
  it('queues panel loads until response bodies finish and releases failed reads', async () => {
    const finishes: (() => void)[] = [];
    const fetcher = vi.fn<typeof fetch>().mockImplementation(
      async () =>
        new Response(
          new ReadableStream({
            start(controller) {
              finishes.push(() => controller.error(new Error('read failed')));
            },
          })
        )
    );
    const data = client(fetcher);
    const first = data.request(request).catch(() => undefined);
    const second = data.request(request).catch(() => undefined);
    const third = data.request(request).catch(() => undefined);
    await Promise.resolve();
    expect(fetcher).toHaveBeenCalledTimes(2);
    finishes[0]!();
    await first;
    expect(fetcher).toHaveBeenCalledTimes(3);
    finishes[1]!();
    finishes[2]!();
    await Promise.all([second, third]);
    fetcher.mockResolvedValue(success());
    await expect(data.request(request)).resolves.toMatchObject({ kind: 'events' });
  });

  it('removes cancelled queued loads and bounds the queue', async () => {
    let release!: () => void;
    const blocked = new Promise<void>((resolve) => {
      release = resolve;
    });
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async () => {
      await blocked;
      return success();
    });
    const data = client(fetcher);
    const running = [data.request(request), data.request(request)];
    const aborts = Array.from({ length: 32 }, () => new AbortController());
    const waiting = aborts.map((abort) => data.request(request, abort.signal).catch((error) => error));
    await expect(data.request(request)).rejects.toThrow('Too many pending');
    aborts.forEach((abort) => abort.abort());
    expect((await Promise.all(waiting)).every((error) => error.name === 'AbortError')).toBe(true);
    release();
    await Promise.all(running);
    expect(fetcher).toHaveBeenCalledTimes(2);
    await data.request(request);
    expect(fetcher).toHaveBeenCalledTimes(3);
  });

  it('retries temporary 503s with backoff and preserves authentication', async () => {
    vi.useFakeTimers();
    const fetcher = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(new Response(null, { status: 503 }))
      .mockResolvedValueOnce(new Response(null, { status: 503 }))
      .mockResolvedValueOnce(success());
    const result = client(fetcher).request(request);
    await vi.advanceTimersByTimeAsync(99);
    expect(fetcher).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1);
    expect(fetcher).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(250);
    await expect(result).resolves.toMatchObject({ kind: 'events' });
    expect(fetcher).toHaveBeenCalledTimes(3);
    expect(fetcher.mock.calls[2]).toEqual(fetcher.mock.calls[0]);
    expect(fetcher.mock.calls[0]![1]!.headers).toMatchObject({ 'X-Float-View-Token': 'token' });
  });

  it('stops after four retries and releases the slot', async () => {
    vi.useFakeTimers();
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async () => new Response(null, { status: 503 }));
    const data = client(fetcher);
    const failures = [0, 1].map(() => expect(data.request(request)).rejects.toThrow('HTTP 503'));
    await vi.runAllTimersAsync();
    await Promise.all(failures);
    expect(fetcher).toHaveBeenCalledTimes(10);
    fetcher.mockImplementation(async () => success());
    await Promise.all([data.request(request), data.request(request)]);
  });

  it('cancels during backoff without another fetch', async () => {
    vi.useFakeTimers();
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async () => new Response(null, { status: 503 }));
    const abort = new AbortController();
    const failure = expect(client(fetcher).request(request, abort.signal)).rejects.toMatchObject({
      name: 'AbortError',
    });
    await vi.advanceTimersByTimeAsync(0);
    abort.abort();
    await failure;
    await vi.runAllTimersAsync();
    expect(fetcher).toHaveBeenCalledTimes(1);
  });

  it('does not send an already cancelled request', async () => {
    const fetcher = vi.fn<typeof fetch>();
    const abort = new AbortController();
    abort.abort();
    await expect(client(fetcher).request(request, abort.signal)).rejects.toMatchObject({ name: 'AbortError' });
    expect(fetcher).not.toHaveBeenCalled();
  });

  it.each([400, 401, 500])('does not retry HTTP %s', async (status) => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(new Response(null, { status }));
    await expect(client(fetcher).request(request)).rejects.toThrow(`HTTP ${status}`);
    expect(fetcher).toHaveBeenCalledTimes(1);
  });
});

it('preserves HTTP conflict status for bounded snapshot refresh', async () => {
  const data = client(async () => new Response('', { status: 409 }));
  await expect(data.request(request)).rejects.toMatchObject({ name: 'SessionDataHttpError', status: 409 });
});
