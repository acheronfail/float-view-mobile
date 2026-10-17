import { reportSessionRequestFailure } from './request-failures';
import { bootstrapSessionDataUrl, type BootstrapDocument } from './bootstrap';
import { MAX_SESSION_DATA_REQUEST_BYTES } from './contract';
import { reportFrontendPerformance } from './performance-log';
import {
  readBoundedJson,
  recordSessionDataDecodedBytes,
  type SessionDataReadTiming,
  validateSessionDataRequest,
  validateSessionDataResult,
  type SessionDataRequest,
  type SessionDataResult,
} from './session-data';

export class SessionDataHttpError extends Error {
  constructor(readonly status: number) {
    super(`Session-data request failed: HTTP ${status}`);
    this.name = 'SessionDataHttpError';
  }
}

function timestamp(): number {
  return globalThis.performance?.now() ?? Date.now();
}

// Shared by every review controller through the connection's SessionDataClient.
// Match the loopback server's MAX_HISTORY_READS; keep pending work bounded too.
const MAX_ACTIVE_REQUESTS = 2;
const MAX_PENDING_REQUESTS = 32;
const RETRY_DELAYS_MS = [100, 250, 500, 1_000];

function retryDelay(milliseconds: number, signal?: AbortSignal): Promise<void> {
  signal?.throwIfAborted();
  return new Promise((resolve, reject) => {
    const cancel = () => {
      clearTimeout(timer);
      signal?.removeEventListener('abort', cancel);
      reject(signal?.reason);
    };
    const timer = setTimeout(() => {
      signal?.removeEventListener('abort', cancel);
      resolve();
    }, milliseconds);
    signal?.addEventListener('abort', cancel, { once: true });
  });
}

export class SessionDataClient {
  private readonly endpoint: URL;
  private activeRequests = 0;
  private readonly pending: (() => void)[] = [];

  private acquire(signal?: AbortSignal): Promise<void> {
    signal?.throwIfAborted();
    if (this.activeRequests < MAX_ACTIVE_REQUESTS) {
      this.activeRequests += 1;
      return Promise.resolve();
    }
    if (this.pending.length >= MAX_PENDING_REQUESTS) {
      return Promise.reject(new Error('Too many pending session-data requests'));
    }
    return new Promise((resolve, reject) => {
      const start = () => {
        signal?.removeEventListener('abort', cancel);
        resolve();
      };
      const cancel = () => {
        const index = this.pending.indexOf(start);
        if (index !== -1) this.pending.splice(index, 1);
        signal?.removeEventListener('abort', cancel);
        reject(signal?.reason);
      };
      this.pending.push(start);
      signal?.addEventListener('abort', cancel, { once: true });
    });
  }

  private release() {
    const next = this.pending.shift();
    if (next) next();
    else this.activeRequests -= 1;
  }

  constructor(
    bootstrap: BootstrapDocument,
    private readonly development = import.meta.env.DEV,
    base = typeof window === 'undefined' ? 'http://127.0.0.1/' : window.location.href,
    private readonly fetcher: typeof fetch = fetch
  ) {
    this.endpoint = bootstrapSessionDataUrl(bootstrap, base, development);
    this.token = bootstrap.sessionToken ?? '';
  }

  private readonly token: string;

  async request(request: SessionDataRequest, signal?: AbortSignal): Promise<SessionDataResult> {
    return reportSessionRequestFailure(async () => {
      const started = timestamp();
      const validated = validateSessionDataRequest(request);
      const body = JSON.stringify(validated);
      if (new TextEncoder().encode(body).byteLength > MAX_SESSION_DATA_REQUEST_BYTES) {
        throw new Error('Session-data request exceeds the body limit');
      }
      await this.acquire(signal);
      try {
        return await this.performRequest(validated, body, started, signal);
      } finally {
        this.release();
      }
    }, signal);
  }

  private async performRequest(
    validated: SessionDataRequest,
    body: string,
    started: number,
    signal?: AbortSignal
  ): Promise<SessionDataResult> {
    let response: Response;
    for (let attempt = 0; ; attempt += 1) {
      signal?.throwIfAborted();
      response = await this.fetcher(this.endpoint, {
        method: 'POST',
        body,
        signal,
        cache: 'no-store',
        headers: {
          Accept: 'application/json',
          'Content-Type': 'application/json',
          'X-Float-View-Token': this.token,
        },
      });
      if (response.status !== 503 || attempt >= RETRY_DELAYS_MS.length) break;
      await response.body?.cancel();
      // Keep the slot during backoff so queued work cannot amplify overload.
      await retryDelay(RETRY_DELAYS_MS[attempt]!, signal);
    }
    signal?.throwIfAborted();
    if (!response.ok) {
      await response.body?.cancel();
      throw new SessionDataHttpError(response.status);
    }
    const readTiming: SessionDataReadTiming = { decodedBytes: 0, bodyMs: 0, parseMs: 0 };
    const value = await readBoundedJson(response, readTiming);
    const validationStarted = timestamp();
    const result = validateSessionDataResult(value, validated);
    reportFrontendPerformance('session-data', {
      kind: validated.kind,
      fields: 'fields' in validated ? validated.fields.length : 0,
      decodedBytes: readTiming.decodedBytes,
      fetchMs: Math.round((validationStarted - started - readTiming.bodyMs - readTiming.parseMs) * 100) / 100,
      bodyMs: Math.round(readTiming.bodyMs * 100) / 100,
      parseMs: Math.round(readTiming.parseMs * 100) / 100,
      validateMs: Math.round((timestamp() - validationStarted) * 100) / 100,
      totalMs: Math.round((timestamp() - started) * 100) / 100,
      observations: Object.values(result.kind === 'series' ? result.columns : {}).reduce(
        (sum, column) => sum + column.values.length,
        0
      ),
    });
    recordSessionDataDecodedBytes(result, readTiming.decodedBytes);
    return result;
  }
}
