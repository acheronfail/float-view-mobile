import { describe, expect, it, vi } from 'vitest';
import { SessionImportClient } from './session-import-client';

describe('session import client', () => {
  it('posts the archive with loopback authentication and returns the new session id', async () => {
    const fetcher = vi.fn<typeof fetch>(async () => new Response(JSON.stringify({ sessionId: 'imported-session' })));
    const client = new SessionImportClient(
      {
        webSocketUrl: 'ws://127.0.0.1:11010/api/ws',
        sessionImportUrl: 'http://127.0.0.1:11010/api/session-import',
        sessionToken: 'token',
      },
      false,
      'http://127.0.0.1:11010/',
      fetcher
    );
    const archive = new Blob(['sqlite archive']);

    await expect(client.upload(archive)).resolves.toEqual({ sessionId: 'imported-session' });
    expect(fetcher).toHaveBeenCalledWith(
      new URL('http://127.0.0.1:11010/api/session-import'),
      expect.objectContaining({
        method: 'POST',
        body: archive,
        headers: expect.objectContaining({
          'Content-Type': 'application/octet-stream',
          'X-Float-View-Token': 'token',
        }),
      })
    );
  });

  it('rejects an archive larger than the import limit before making a request', async () => {
    const fetcher = vi.fn<typeof fetch>();
    const client = new SessionImportClient(
      { webSocketUrl: 'ws://127.0.0.1:11010/api/ws', sessionToken: 'token' },
      false,
      'http://127.0.0.1:11010/',
      fetcher
    );
    const archive = { size: 128 * 1024 * 1024 + 1 } as Blob;

    await expect(client.upload(archive)).rejects.toThrow('128 MiB');
    expect(fetcher).not.toHaveBeenCalled();
  });
});

it('uploads app ZIPs with an explicit preference choice and validates counts', async () => {
  const summary = {
    boardsAdded: 1,
    boardsSkipped: 0,
    sessionsAdded: 3,
    sessionsSkipped: 2,
    tunesAdded: 0,
    tunesSkipped: 1,
    settingsImported: true,
  };
  const fetcher = vi.fn<typeof fetch>(async () => new Response(JSON.stringify(summary)));
  const client = new SessionImportClient(
    { webSocketUrl: 'ws://127.0.0.1/api/ws', sessionToken: 'token' },
    false,
    'http://127.0.0.1/',
    fetcher
  );
  const file = new Blob(['app zip']);
  await expect(client.uploadApp(file, true)).resolves.toEqual(summary);
  expect(fetcher).toHaveBeenCalledWith(
    new URL('http://127.0.0.1/api/app-import'),
    expect.objectContaining({
      body: file,
      headers: expect.objectContaining({ 'X-Float-View-Token': 'token', 'X-Float-View-Import-Settings': 'true' }),
    })
  );
  await expect(client.uploadApp({ size: 2 * 1024 * 1024 * 1024 + 1 } as Blob, false)).rejects.toThrow('2 GiB');
  expect(fetcher).toHaveBeenCalledTimes(1);
});
