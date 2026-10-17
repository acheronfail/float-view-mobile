import { describe, expect, it } from 'vitest';
import { RequestFailureReporter } from './request-failures';
import { ApiClient } from './client';

describe('request failure reporting', () => {
  it('queues at most ten closed events across disconnects and retries without recursion', async () => {
    let now = 0;
    const reporter = new RequestFailureReporter(() => now);
    for (let i = 0; i < 30; i++) reporter.record('protocol');
    const sent: unknown[] = [];
    const detach = reporter.attach(async () => {
      throw new Error('PRIVATE');
    });
    await new Promise((resolve) => setTimeout(resolve, 0));
    detach();
    reporter.attach(async (failure) => {
      sent.push(failure);
    });
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(sent).toHaveLength(10);
    expect(sent[0]).toEqual({ category: 'protocol', feature: 'app', line: null, column: null });
    now = 60_001;
    reporter.record('request', 'sessions');
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(sent).toHaveLength(11);
    expect(JSON.stringify(sent)).not.toContain('PRIVATE');
  });

  it('captures caught timeouts but never reports error envelopes or diagnostic failures twice', async () => {
    const reporter = new RequestFailureReporter();
    const socket = new EventTarget() as EventTarget & {
      readyState: number;
      send: (payload: string) => void;
      close: () => void;
    };
    socket.readyState = 1;
    const sent: { id: number; method: string }[] = [];
    socket.send = (payload) => {
      const request = JSON.parse(payload);
      sent.push(request);
      if (request.method === 'diagnostics.uiError')
        queueMicrotask(() =>
          socket.dispatchEvent(
            new MessageEvent('message', {
              data: JSON.stringify({ id: request.id, error: { code: 'unavailable', message: 'PRIVATE' } }),
            })
          )
        );
    };
    socket.close = () => {};
    const client = new ApiClient(socket as unknown as WebSocket, 5, reporter);
    await expect(client.request('state.get', {})).rejects.toThrow('timed out');
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(sent.map((r) => r.method)).toEqual(['state.get', 'diagnostics.uiError']);
    const result = client.request('refloat.tune.apply', { tuneId: 'PRIVATE', expectedRevision: 1, baseRevision: 1 });
    const id = sent.at(-1)!.id;
    socket.dispatchEvent(
      new MessageEvent('message', { data: JSON.stringify({ id, error: { code: 'conflict', message: 'PRIVATE' } }) })
    );
    await expect(result).rejects.toMatchObject({ code: 'conflict' });
    // One retry of the queued diagnostic on the ordinary request, no recursive submissions.
    expect(sent.filter((r) => r.method === 'diagnostics.uiError')).toHaveLength(2);
    client.close();
  });
});

it('captures HTTP/body failures without private text and ignores deliberately cancelled reads', async () => {
  const { requestFailures, reportSessionRequestFailure } = await import('./request-failures');
  const sent: unknown[] = [];
  const detach = requestFailures.attach(async (failure) => {
    sent.push(failure);
  });
  await expect(
    reportSessionRequestFailure(async () => {
      throw new Error('PRIVATE /path token tune');
    })
  ).rejects.toThrow('PRIVATE');
  const controller = new AbortController();
  controller.abort();
  await expect(
    reportSessionRequestFailure(async () => {
      throw controller.signal.reason;
    }, controller.signal)
  ).rejects.toBeDefined();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(sent).toEqual([{ category: 'request', feature: 'sessions', line: null, column: null }]);
  expect(JSON.stringify(sent)).not.toContain('PRIVATE');
  detach();
});
