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

function timestamp(): number {
  return globalThis.performance?.now() ?? Date.now();
}

export class SessionDataClient {
  private readonly endpoint: URL;

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
    const started = timestamp();
    const validated = validateSessionDataRequest(request);
    const body = JSON.stringify(validated);
    if (new TextEncoder().encode(body).byteLength > MAX_SESSION_DATA_REQUEST_BYTES) {
      throw new Error('Session-data request exceeds the body limit');
    }
    const response = await this.fetcher(this.endpoint, {
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
    if (!response.ok) throw new Error(`Session-data request failed: HTTP ${response.status}`);
    const readTiming: SessionDataReadTiming = { decodedBytes: 0, bodyMs: 0, parseMs: 0 };
    const value = await readBoundedJson(response, readTiming);
    const validationStarted = timestamp();
    const result = validateSessionDataResult(value, validated);
    reportFrontendPerformance('session-data', {
      kind: request.kind,
      fields: 'fields' in request ? request.fields.length : 0,
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
