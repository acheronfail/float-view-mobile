import { describe, expect, it } from 'vitest';
import type { AppState } from '$lib/backend/contract';
import { ApiClient, ApiProtocolError } from './client';

type Listener = (event: { data?: unknown }) => void;

class FakeWebSocket {
  readyState = 1;
  readonly sent: string[] = [];
  private listeners = new Map<string, Set<Listener>>();

  addEventListener(type: string, listener: Listener) {
    const listeners = this.listeners.get(type) ?? new Set();
    listeners.add(listener);
    this.listeners.set(type, listeners);
  }
  removeEventListener(type: string, listener: Listener) {
    this.listeners.get(type)?.delete(listener);
  }
  send(data: string) {
    this.sent.push(data);
  }
  close() {
    this.readyState = 3;
    this.emit('close', {});
  }
  receive(message: unknown) {
    this.emit('message', { data: JSON.stringify(message) });
  }
  receiveRaw(data: string) {
    this.emit('message', { data });
  }
  private emit(type: string, event: { data?: unknown }) {
    for (const listener of this.listeners.get(type) ?? []) listener(event);
  }
}

function setup() {
  const socket = new FakeWebSocket();
  return { socket, client: new ApiClient(socket as unknown as WebSocket, 20) };
}

describe('ApiClient', () => {
  it('sends the small request envelope and resolves exactly once', async () => {
    const { socket, client } = setup();
    const result = client.request('state.get', {});
    expect(JSON.parse(socket.sent[0])).toEqual({ id: 1, method: 'state.get', params: {} });
    socket.receive({ id: 1, result: { revision: 3 } });
    socket.receive({ id: 1, result: { revision: 4 } });
    await expect(result).resolves.toMatchObject({ revision: 3 });
    client.close();
  });

  it('exposes stable string errors', async () => {
    const { socket, client } = setup();
    const result = client.request('state.get', {});
    socket.receive({ id: 1, error: { code: 'invalid_state', message: 'Not ready' } });
    await expect(result).rejects.toEqual(expect.objectContaining({ code: 'invalid_state', message: 'Not ready' }));
    client.close();
  });

  it('publishes authoritative state events and supports unsubscribe', () => {
    const { socket, client } = setup();
    const revisions: number[] = [];
    const unsubscribe = client.onStateChanged((state) => revisions.push(state.revision));
    socket.receive({ event: 'state.changed', data: { revision: 1 } as AppState });
    unsubscribe();
    socket.receive({ event: 'state.changed', data: { revision: 2 } as AppState });
    expect(revisions).toEqual([1]);
    client.close();
  });

  it('rejects pending work on close, timeout, and invalid messages', async () => {
    const closed = setup();
    const closedResult = closed.client.request('state.get', {});
    closed.socket.close();
    await expect(closedResult).rejects.toBeInstanceOf(ApiProtocolError);

    const timed = setup();
    await expect(timed.client.request('state.get', {})).rejects.toThrow('timed out');

    const malformed = setup();
    const malformedResult = malformed.client.request('state.get', {});
    malformed.socket.receiveRaw('{');
    await expect(malformedResult).rejects.toThrow('invalid JSON');
  });
});
