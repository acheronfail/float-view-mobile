import { validatePermissions } from './permissions';
import type {
  AppState,
  BackendError,
  BackendEvent,
  BackendMethod,
  BackendParams,
  BackendResult,
  Response,
} from '$lib/backend/contract';
import { MAX_MESSAGE_BYTES } from '$lib/backend/contract';

type PendingRequest = {
  method: BackendMethod;
  resolve: (value: unknown) => void;
  reject: (reason: unknown) => void;
  timeout: ReturnType<typeof setTimeout>;
};

export class ApiRequestError extends Error {
  constructor(readonly error: BackendError) {
    super(error.message);
    this.name = 'ApiRequestError';
  }

  get code() {
    return this.error.code;
  }
}

export class ApiProtocolError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ApiProtocolError';
  }
}

export class ApiClient {
  private nextId = 1;
  private readonly pending = new Map<number, PendingRequest>();
  private readonly stateHandlers = new Set<(state: AppState) => void>();
  private readonly closedHandlers = new Set<() => void>();
  private closed = false;
  private diagnosticsAt = performance.now();
  private lastSnapshotAt = this.diagnosticsAt;
  private deliveryDiagnostics = { snapshots: 0, max_sample_age_ms: 0, max_delivery_gap_ms: 0, max_handler_ms: 0 };

  constructor(
    private readonly socket: WebSocket,
    private readonly defaultTimeoutMs = 5_000
  ) {
    socket.addEventListener('message', this.handleMessage);
    socket.addEventListener('close', this.handleClose);
  }

  get isOpen(): boolean {
    return !this.closed && this.socket.readyState === 1;
  }

  request<M extends BackendMethod>(
    method: M,
    params: BackendParams<M>,
    timeoutMs = this.defaultTimeoutMs
  ): Promise<BackendResult<M>> {
    if (!this.isOpen) return Promise.reject(new ApiProtocolError('Backend socket is not open'));
    const id = this.nextId++;
    const payload = JSON.stringify({ id, method, params });
    if (new TextEncoder().encode(payload).byteLength > MAX_MESSAGE_BYTES) {
      return Promise.reject(new ApiProtocolError('Backend request exceeds the message limit'));
    }
    return new Promise<BackendResult<M>>((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pending.delete(id);
        reject(new ApiProtocolError(`Backend request timed out: ${method}`));
      }, timeoutMs);
      this.pending.set(id, { method, resolve: resolve as (value: unknown) => void, reject, timeout });
      try {
        this.socket.send(payload);
      } catch (error) {
        clearTimeout(timeout);
        this.pending.delete(id);
        reject(error);
      }
    });
  }

  onStateChanged(handler: (state: AppState) => void): () => void {
    this.stateHandlers.add(handler);
    return () => this.stateHandlers.delete(handler);
  }

  onClosed(handler: () => void): () => void {
    this.closedHandlers.add(handler);
    return () => this.closedHandlers.delete(handler);
  }

  private notifyClosed() {
    for (const handler of this.closedHandlers) handler();
    this.closedHandlers.clear();
  }

  close(code = 1000, reason = 'Client closed'): void {
    if (this.closed) return;
    this.closed = true;
    this.notifyClosed();
    this.detach();
    this.rejectPending(new ApiProtocolError(reason));
    this.socket.close(code, reason);
  }

  private readonly handleMessage = (event: MessageEvent<unknown>): void => {
    if (typeof event.data !== 'string' || new TextEncoder().encode(event.data).byteLength > MAX_MESSAGE_BYTES) {
      this.failProtocol('Backend sent an invalid or oversized message');
      return;
    }
    let value: unknown;
    try {
      value = JSON.parse(event.data);
    } catch {
      this.failProtocol('Backend sent invalid JSON');
      return;
    }
    if (!value || typeof value !== 'object') {
      this.failProtocol('Backend sent an invalid envelope');
      return;
    }
    const envelope = value as Partial<Response & BackendEvent>;
    if (envelope.event === 'state.changed') {
      if (!envelope.data || typeof envelope.data.revision !== 'number') {
        this.failProtocol('Backend sent an invalid state event');
        return;
      }
      try {
        validatePermissions(envelope.data.permissions);
      } catch {
        this.failProtocol('Backend sent an invalid permission inventory');
        return;
      }
      const started = performance.now();
      for (const handler of this.stateHandlers) handler(envelope.data);
      const now = performance.now();
      const stats = this.deliveryDiagnostics;
      stats.snapshots = Math.min(10_000, stats.snapshots + 1);
      stats.max_delivery_gap_ms = Math.min(
        86_400_000,
        Math.max(stats.max_delivery_gap_ms, Math.round(started - this.lastSnapshotAt))
      );
      stats.max_handler_ms = Math.min(60_000, Math.max(stats.max_handler_ms, Math.round(now - started)));
      const observed = Date.parse(envelope.data.activeConnection?.latestSample?.observedAt ?? '');
      if (Number.isFinite(observed))
        stats.max_sample_age_ms = Math.min(86_400_000, Math.max(stats.max_sample_age_ms, Date.now() - observed, 0));
      this.lastSnapshotAt = started;
      if (now - this.diagnosticsAt >= 10_000) {
        this.diagnosticsAt = now;
        this.deliveryDiagnostics = { snapshots: 0, max_sample_age_ms: 0, max_delivery_gap_ms: 0, max_handler_ms: 0 };
        void this.request('diagnostics.report', stats).catch(() => {
          /* Diagnostics never disrupt the client. */
        });
      }
      return;
    }
    if (!Number.isSafeInteger(envelope.id)) {
      this.failProtocol('Backend response has no valid request ID');
      return;
    }
    const request = this.pending.get(envelope.id!);
    if (!request) return;
    this.pending.delete(envelope.id!);
    clearTimeout(request.timeout);
    if ('error' in envelope && envelope.error) request.reject(new ApiRequestError(envelope.error));
    else if ('result' in envelope) {
      try {
        if (request.method === 'state.get') validatePermissions((envelope.result as AppState)?.permissions);
        if (request.method === 'permissions.recover') validatePermissions(envelope.result);
        request.resolve(envelope.result);
      } catch {
        request.reject(new ApiProtocolError('Backend sent an invalid permission inventory'));
        this.close(1002, 'Invalid permission inventory');
      }
    } else request.reject(new ApiProtocolError('Backend response has no result or error'));
  };

  private failProtocol(message: string) {
    const error = new ApiProtocolError(message);
    this.rejectPending(error);
    this.close(1002, message);
  }

  private readonly handleClose = (): void => {
    if (this.closed) return;
    this.closed = true;
    this.notifyClosed();
    this.detach();
    this.rejectPending(new ApiProtocolError('Backend socket closed'));
  };

  private detach() {
    this.socket.removeEventListener('message', this.handleMessage);
    this.socket.removeEventListener('close', this.handleClose);
  }

  private rejectPending(error: Error) {
    for (const request of this.pending.values()) {
      clearTimeout(request.timeout);
      request.reject(error);
    }
    this.pending.clear();
  }
}

export function connectApiClient(
  url: string,
  timeoutMs = 5_000,
  createSocket: (url: string) => WebSocket = (socketUrl) => new WebSocket(socketUrl)
): Promise<ApiClient> {
  return new Promise((resolve, reject) => {
    const socket = createSocket(url);
    const timeout = setTimeout(() => {
      cleanup();
      socket.close();
      reject(new ApiProtocolError('Timed out opening backend socket'));
    }, timeoutMs);
    const cleanup = () => {
      clearTimeout(timeout);
      socket.removeEventListener('open', handleOpen);
      socket.removeEventListener('error', handleError);
    };
    const handleOpen = () => {
      cleanup();
      resolve(new ApiClient(socket, timeoutMs));
    };
    const handleError = () => {
      cleanup();
      socket.close();
      reject(new ApiProtocolError('Failed to open backend socket'));
    };
    socket.addEventListener('open', handleOpen);
    socket.addEventListener('error', handleError);
  });
}
