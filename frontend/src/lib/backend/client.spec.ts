import { permissionInventory } from '../../../backend-mock/permissions';
import { describe, expect, it, vi } from 'vitest';
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
    socket.receive({ id: 1, result: { revision: 3, permissions: permissionInventory('ios') } });
    socket.receive({ id: 1, result: { revision: 4, permissions: permissionInventory('ios') } });
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
    socket.receive({
      event: 'state.changed',
      data: { revision: 1, permissions: permissionInventory('ios') } as AppState,
    });
    unsubscribe();
    socket.receive({
      event: 'state.changed',
      data: { revision: 2, permissions: permissionInventory('ios') } as AppState,
    });
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

  it('measures incoming control messages as UTF-8 bytes', async () => {
    const { socket, client } = setup();
    const result = client.request('state.get', {});
    socket.receiveRaw('😀'.repeat(16_385));
    await expect(result).rejects.toThrow('oversized message');
  });
});

it('closes malformed permission inventories and notifies closure once', async () => {
  const { client, socket } = setup();
  let closed = 0;
  client.onClosed(() => closed++);
  const pending = client.request('state.get', {});
  socket.receive({ id: 1, result: { revision: 1, permissions: [{ permission: 'arbitrary' }] } });
  await expect(pending).rejects.toThrow('invalid permission inventory');
  client.close();
  expect(closed).toBe(1);
});

it('reports bounded delivery timings at most once per ten seconds without revising state', async () => {
  let now = 0;
  const clock = vi.spyOn(performance, 'now').mockImplementation(() => now);
  const { socket, client } = setup();
  const state = { revision: 1, permissions: permissionInventory('ios') } as AppState;
  try {
    socket.receive({ event: 'state.changed', data: state });
    expect(socket.sent).toHaveLength(0);
    now = 10000;
    socket.receive({ event: 'state.changed', data: state });
    expect(socket.sent).toHaveLength(1);
    const report = JSON.parse(socket.sent[0]!);
    expect(report.method).toBe('diagnostics.report');
    expect(report.params.snapshots).toBe(2);
    expect(report.params.max_delivery_gap_ms).toBe(10000);
    socket.receive({ id: report.id, error: { code: 'unavailable', message: 'Unavailable' } });
    await Promise.resolve();
    expect(client.isOpen).toBe(true);
    now = 10001;
    socket.receive({ event: 'state.changed', data: state });
    expect(socket.sent).toHaveLength(1);
  } finally {
    client.close();
    clock.mockRestore();
  }
});
