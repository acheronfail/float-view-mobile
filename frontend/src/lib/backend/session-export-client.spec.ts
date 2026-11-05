import { describe, expect, it, vi } from 'vitest';
import { SessionExportClient } from './session-export-client';

describe('session export download client', () => {
  it('falls back to a compressed CSV filename without response metadata', async () => {
    const client = new SessionExportClient(
      { webSocketUrl: 'ws://127.0.0.1/api/ws' },
      true,
      'http://127.0.0.1/',
      vi.fn<typeof fetch>(async () => new Response('zip bytes'))
    );
    expect((await client.download('session-1', 'csv')).fileName).toBe('float-view-session-1.csv.zip');
  });

  it('posts authenticated export requests and uses the response filename', async () => {
    const fetcher = vi.fn<typeof fetch>(
      async () =>
        new Response('csv body', {
          status: 200,
          headers: { 'Content-Disposition': 'attachment; filename="ride.csv.zip"' },
        })
    );
    const client = new SessionExportClient(
      {
        webSocketUrl: 'ws://127.0.0.1:11003/api/ws',
        sessionExportUrl: 'http://127.0.0.1:11003/api/session-export',
        sessionToken: 'token',
      },
      false,
      'http://127.0.0.1:11003/',
      fetcher
    );

    const download = await client.download('session-1', 'csv');

    expect(download.fileName).toBe('ride.csv.zip');
    expect(await download.blob.text()).toBe('csv body');
    expect(fetcher).toHaveBeenCalledWith(
      new URL('http://127.0.0.1:11003/api/session-export'),
      expect.objectContaining({
        method: 'POST',
        headers: expect.objectContaining({ 'X-Float-View-Token': 'token' }),
        body: JSON.stringify({ sessionId: 'session-1', format: 'csv' }),
      })
    );
  });
});

describe('board export download client', () => {
  it('sends a board selector and defaults to a ZIP filename', async () => {
    const fetcher = vi.fn<typeof fetch>(async () => new Response('bundle'));
    const client = new SessionExportClient(
      { webSocketUrl: 'ws://127.0.0.1/api/ws' },
      true,
      'http://127.0.0.1/',
      fetcher
    );
    const output = await client.downloadBoard('board-1', 'float-view-archive');
    expect(output.fileName).toMatch(/\.zip$/);
    expect(fetcher).toHaveBeenCalledWith(
      expect.any(URL),
      expect.objectContaining({
        body: JSON.stringify({ savedDeviceId: 'board-1', format: 'float-view-archive' }),
      })
    );
  });
});

it('correlates a board download and forwards delivery cancellation', async () => {
  const abort = new AbortController();
  const fetcher = vi.fn<typeof fetch>(async () => new Response('bundle'));
  const client = new SessionExportClient({ webSocketUrl: 'ws://127.0.0.1/api/ws' }, true, 'http://127.0.0.1/', fetcher);
  await client.downloadBoard('board-1', 'csv', 'export-operation', abort.signal);
  expect(fetcher).toHaveBeenCalledWith(
    expect.any(URL),
    expect.objectContaining({
      signal: abort.signal,
      body: JSON.stringify({ savedDeviceId: 'board-1', format: 'csv', operationId: 'export-operation' }),
    })
  );
});
