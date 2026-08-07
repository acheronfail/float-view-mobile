import WebSocket from 'ws';
import type {
  AppState,
  BackendError,
  BackendMethod,
  BackendParams,
  BackendResult,
} from '../../frontend/src/lib/backend/contract.ts';
import { MAX_MESSAGE_BYTES } from '../../frontend/src/lib/backend/contract.ts';
import { MAX_SESSION_DATA_REQUEST_BYTES } from '../../frontend/src/lib/backend/contract.ts';
import {
  readBoundedJson,
  validateSessionDataRequest,
  validateSessionDataResult,
  type SessionDataRequest,
  type SessionDataResult,
} from '../../frontend/src/lib/backend/session-data.ts';

export class ConformanceError extends Error {
  readonly error: BackendError;

  constructor(error: BackendError) {
    super(error.message);
    this.error = error;
  }
}

export class ConformanceClient {
  private readonly socket: WebSocket;
  private readonly origin: string;
  private nextId = 1;
  private state?: AppState;
  private readonly stateWaiters = new Set<{
    predicate: (state: AppState) => boolean;
    resolve: (state: AppState) => void;
    reject: (error: Error) => void;
    timer: NodeJS.Timeout;
  }>();
  private readonly pending = new Map<
    number,
    {
      method: string;
      resolve: (value: unknown) => void;
      reject: (error: Error) => void;
      timer: NodeJS.Timeout;
    }
  >();

  private constructor(socket: WebSocket, origin: string) {
    this.socket = socket;
    this.origin = origin;
    socket.on('message', (data) => this.receive(data.toString()));
    socket.on('close', (code, reason) =>
      this.rejectAll(new Error(`Backend closed the socket (code=${code}, reason=${reason.toString() || 'none'})`))
    );
  }

  private dataEndpoint(): { url: string; token: string } {
    const value = new URL(this.socket.url);
    const token = value.searchParams.get('token') ?? '';
    value.protocol = value.protocol === 'wss:' ? 'https:' : 'http:';
    value.pathname = '/api/session-data';
    value.search = '';
    return { url: value.toString(), token };
  }

  async sessionData(request: SessionDataRequest): Promise<SessionDataResult> {
    const validated = validateSessionDataRequest(request);
    const body = JSON.stringify(validated);
    if (Buffer.byteLength(body, 'utf8') > MAX_SESSION_DATA_REQUEST_BYTES)
      throw new Error('Session-data request exceeded bound');
    const endpoint = this.dataEndpoint();
    const response = await fetch(endpoint.url, {
      method: 'POST',
      body,
      headers: {
        Origin: this.origin,
        'X-Float-View-Token': endpoint.token,
        'Content-Type': 'application/json',
        Accept: 'application/json',
        'Accept-Encoding': 'gzip',
      },
    });
    if (!response.ok) throw new Error(`Session-data request failed: HTTP ${response.status}`);
    return validateSessionDataResult(await readBoundedJson(response), validated);
  }

  static open(url: string, origin: string, timeoutMs = 5_000): Promise<ConformanceClient> {
    return new Promise((resolve, reject) => {
      const socket = new WebSocket(url, { origin, maxPayload: MAX_MESSAGE_BYTES });
      const timer = setTimeout(() => {
        socket.terminate();
        reject(new Error('Timed out connecting to backend'));
      }, timeoutMs);
      socket.once('open', () => {
        clearTimeout(timer);
        resolve(new ConformanceClient(socket, origin));
      });
      socket.once('error', (error) => {
        clearTimeout(timer);
        reject(error);
      });
    });
  }

  request<M extends BackendMethod>(method: M, params: BackendParams<M>, timeoutMs = 5_000): Promise<BackendResult<M>> {
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`Timed out: ${method}`));
      }, timeoutMs);
      this.pending.set(id, { method, resolve: resolve as (value: unknown) => void, reject, timer });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }

  currentState() {
    return this.state;
  }

  waitForState(
    predicate: (state: AppState) => boolean,
    options: { afterRevision?: number; timeoutMs?: number } = {}
  ): Promise<AppState> {
    const afterRevision = options.afterRevision ?? -1;
    const matches = (state: AppState) => state.revision > afterRevision && predicate(state);
    if (this.state && matches(this.state)) return Promise.resolve(this.state);
    if (this.stateWaiters.size >= 32) {
      return Promise.reject(new Error('Too many pending state waits'));
    }
    return new Promise((resolve, reject) => {
      const waiter = {
        predicate: matches,
        resolve,
        reject,
        timer: setTimeout(() => {
          this.stateWaiters.delete(waiter);
          reject(new Error(`Timed out waiting for authoritative state; latest=${JSON.stringify(this.state)}`));
        }, options.timeoutMs ?? 5_000),
      };
      this.stateWaiters.add(waiter);
    });
  }

  close() {
    this.socket.close();
  }

  private receive(text: string) {
    if (Buffer.byteLength(text, 'utf8') > MAX_MESSAGE_BYTES) {
      throw new Error('Backend message exceeded bound');
    }
    const message = JSON.parse(text) as Record<string, unknown>;
    if (message.event === 'state.changed') {
      const state = message.data as AppState;
      if (process.env.FLOAT_VIEW_CONFORMANCE_TRACE === '1') {
        console.error(`state.changed revision=${state.revision}`);
      }
      if (!this.state || state.revision >= this.state.revision) {
        this.state = state;
        this.resolveStateWaiters(state);
      }
      return;
    }
    const id = message.id;
    if (!Number.isSafeInteger(id)) throw new Error('Response has invalid ID');
    const pending = this.pending.get(id as number);
    if (!pending) return;
    if (process.env.FLOAT_VIEW_CONFORMANCE_TRACE === '1') {
      console.error(`response method=${pending.method} id=${String(id)}`);
    }
    this.pending.delete(id as number);
    clearTimeout(pending.timer);
    if (message.error) pending.reject(new ConformanceError(message.error as BackendError));
    else if ('result' in message) {
      if (pending.method === 'state.get') {
        const state = message.result as AppState;
        if (!this.state || state.revision >= this.state.revision) {
          this.state = state;
          this.resolveStateWaiters(state);
        }
      }
      pending.resolve(message.result);
    } else pending.reject(new Error('Response has no result or error'));
  }

  private rejectAll(error: Error) {
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(new Error(`${error.message} while waiting for ${pending.method}`));
    }
    this.pending.clear();
    for (const waiter of this.stateWaiters) {
      clearTimeout(waiter.timer);
      waiter.reject(
        new Error(`${error.message} while waiting for authoritative state; latest=${JSON.stringify(this.state)}`)
      );
    }
    this.stateWaiters.clear();
  }

  private resolveStateWaiters(state: AppState) {
    for (const waiter of this.stateWaiters) {
      if (!waiter.predicate(state)) continue;
      clearTimeout(waiter.timer);
      this.stateWaiters.delete(waiter);
      waiter.resolve(state);
    }
  }
}
