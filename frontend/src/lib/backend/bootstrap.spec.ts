import { describe, expect, it } from 'vitest';
import { bootstrapSessionDataUrl, bootstrapSessionImportUrl, bootstrapWebSocketUrl } from './bootstrap';

describe('bootstrap WebSocket endpoint', () => {
  it('routes native development sockets through the Vite origin', () => {
    expect(
      bootstrapWebSocketUrl(
        {
          webSocketUrl: 'ws://127.0.0.1:11010/api/ws?existing=yes',
          sessionToken: 'launch token',
        },
        'http://127.0.0.1:11002/',
        true
      ).toString()
    ).toBe('ws://127.0.0.1:11002/api/ws?existing=yes&token=launch+token');
  });

  it('preserves the backend endpoint in production', () => {
    expect(
      bootstrapWebSocketUrl(
        { webSocketUrl: 'ws://127.0.0.1:11011/api/ws', sessionToken: 'token' },
        'http://127.0.0.1:11011/',
        false
      ).toString()
    ).toBe('ws://127.0.0.1:11011/api/ws?token=token');
  });

  it('routes authenticated session-data requests through the development origin', () => {
    expect(
      bootstrapSessionDataUrl(
        { webSocketUrl: 'ws://127.0.0.1:11011/api/ws', sessionDataUrl: 'http://127.0.0.1:11011/api/session-data' },
        'http://127.0.0.1:11002/',
        true
      ).toString()
    ).toBe('http://127.0.0.1:11002/api/session-data');
  });

  it('routes authenticated session imports through the development origin', () => {
    expect(
      bootstrapSessionImportUrl(
        { webSocketUrl: 'ws://127.0.0.1:11011/api/ws', sessionImportUrl: 'http://127.0.0.1:11011/api/session-import' },
        'http://127.0.0.1:11002/',
        true
      ).toString()
    ).toBe('http://127.0.0.1:11002/api/session-import');
  });
});
