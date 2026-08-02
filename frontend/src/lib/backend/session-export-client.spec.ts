import { describe, expect, it, vi } from 'vitest';
import { SessionExportClient } from './session-export-client';

describe('session export download client', () => {
  it('posts authenticated export requests and uses the response filename', async () => {
    const fetcher = vi.fn<typeof fetch>(
      async () =>
        new Response('csv body', {
          status: 200,
          headers: { 'Content-Disposition': 'attachment; filename="ride.csv"' },
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

    expect(download.fileName).toBe('ride.csv');
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
