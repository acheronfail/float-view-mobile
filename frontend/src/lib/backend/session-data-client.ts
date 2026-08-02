import { bootstrapSessionDataUrl, type BootstrapDocument } from './bootstrap';
import { MAX_SESSION_DATA_REQUEST_BYTES } from './contract';
import {
  readBoundedJson,
  validateSessionDataRequest,
  validateSessionDataResult,
  type SessionDataRequest,
  type SessionDataResult,
} from './session-data';

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
    return validateSessionDataResult(await readBoundedJson(response), validated);
  }
}
