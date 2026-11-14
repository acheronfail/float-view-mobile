import { describe, expect, it, vi } from 'vitest';
import { MockBackend } from '../../../backend-mock/api-v1';
import type { BackendMethod, BackendParams, BackendResult } from './contract';
import { ApiRequestError, type ApiClient } from './client';
import { BackendState } from './state.svelte';

function fixture() {
  const backend = new MockBackend(undefined, { safetyDisclaimerAccepted: true });
  const prepares: Array<Record<string, unknown>> = [];
  let owner = 0;
  let release: (() => void) | undefined;
  let hold = false;
  let failedMethod: BackendMethod | null = null;
  let memoryOnly = false;
  const store = new BackendState(
    async () => ({ webSocketUrl: 'ws://test' }),
    async () => {
      const id = ++owner;
      let closed: (() => void) | undefined;
      let open = true;
      return {
        get isOpen() {
          return open;
        },
        async request<M extends BackendMethod>(method: M, params: BackendParams<M>): Promise<BackendResult<M>> {
          if (method === failedMethod) throw new ApiRequestError({ code: 'unavailable', message: 'PRIVATE_ERROR' });
          if (method === 'diagnostics.list' && memoryOnly)
            return { persistence: 'memoryOnly', droppedRecords: 0, incidents: [] } as BackendResult<M>;
          if (method === 'diagnostics.prepare') {
            prepares.push({ ...params });
            if (hold)
              await new Promise<void>((resolve) => {
                release = resolve;
              });
          }
          const reply = backend.handle({ id: 1, method, params }, id);
          if ('error' in reply) throw new ApiRequestError(reply.error);
          return reply.result as BackendResult<M>;
        },
        onStateChanged: () => () => {},
        onClosed: (handler: () => void) => {
          closed = handler;
          return () => {
            closed = undefined;
          };
        },
        close() {
          open = false;
          backend.disconnectClient(id);
          closed?.();
        },
      } as unknown as ApiClient;
    }
  );
  return {
    store,
    prepares,
    hold: () => {
      hold = true;
    },
    release: () => release?.(),
    failAt: (method: BackendMethod) => {
      failedMethod = method;
    },
    memoryOnly: () => {
      memoryOnly = true;
    },
  };
}

describe('connection-owned diagnostic export', () => {
  it('replaces reports beyond the service capacity and forgets handles on reconnect', async () => {
    const { store, prepares } = fixture();
    await store.connect();
    for (let n = 0; n < 4; n++) expect(await store.exportDiagnostics()).toMatchObject({ delivery: 'presented' });
    expect(prepares).toHaveLength(4);
    expect(prepares[0]).not.toHaveProperty('replaceReportId');
    for (const next of prepares.slice(1)) expect(next).toHaveProperty('replaceReportId');
    await store.connect();
    expect(await store.exportDiagnostics()).toMatchObject({ delivery: 'presented' });
    expect(prepares.at(-1)).not.toHaveProperty('replaceReportId');
    store.close();
  });
  it('shares in-flight export work across view recreation', async () => {
    const { store, prepares, hold, release } = fixture();
    await store.connect();
    hold();
    const first = store.exportDiagnostics();
    const second = store.exportDiagnostics();
    await vi.waitFor(() => expect(prepares).toHaveLength(1));
    release();
    expect(await first).toMatchObject({ delivery: 'presented' });
    expect(await second).toMatchObject({ delivery: 'presented' });
    expect(prepares).toHaveLength(1);
    store.close();
  });
});

it.each([
  ['diagnostics.list', 'read diagnostic status'],
  ['diagnostics.prepare', 'create the diagnostic file'],
  ['diagnostics.export', 'open diagnostic sharing'],
] as const)('identifies %s failures without reflecting private error text', async (method, stage) => {
  const { store, failAt } = fixture();
  await store.connect();
  failAt(method);
  await expect(store.exportDiagnostics()).rejects.toThrow(`Could not ${stage} (unavailable).`);
  store.close();
});
it('explains unavailable diagnostic storage before attempting a file', async () => {
  const { store, memoryOnly, prepares } = fixture();
  await store.connect();
  memoryOnly();
  await expect(store.exportDiagnostics()).rejects.toThrow('Could not open saved diagnostics.');
  expect(prepares).toHaveLength(0);
  store.close();
});
