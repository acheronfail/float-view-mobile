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
  private closed = false;

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
      this.pending.set(id, { resolve: resolve as (value: unknown) => void, reject, timeout });
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

  close(code = 1000, reason = 'Client closed'): void {
    if (this.closed) return;
    this.closed = true;
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
      for (const handler of this.stateHandlers) handler(envelope.data);
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
    else if ('result' in envelope) request.resolve(envelope.result);
    else request.reject(new ApiProtocolError('Backend response has no result or error'));
  };

  private failProtocol(message: string) {
    const error = new ApiProtocolError(message);
    this.rejectPending(error);
    this.close(1002, message);
  }

  private readonly handleClose = (): void => {
    if (this.closed) return;
    this.closed = true;
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
