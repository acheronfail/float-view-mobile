import { reportSessionRequestFailure } from './request-failures';
import { bootstrapSessionImportUrl, type BootstrapDocument } from './bootstrap';
import { MAX_SESSION_IMPORT_BYTES } from './contract';
import { reportFrontendPerformance } from './performance-log';

export type SessionImportResult = {
  sessionId: string;
};

export class SessionImportClient {
  private readonly endpoint: URL;
  private readonly token: string;

  constructor(
    bootstrap: BootstrapDocument,
    private readonly development = import.meta.env.DEV,
    base = typeof window === 'undefined' ? 'http://127.0.0.1/' : window.location.href,
    private readonly fetcher: typeof fetch = fetch
  ) {
    this.endpoint = bootstrapSessionImportUrl(bootstrap, base, development);
    this.token = bootstrap.sessionToken ?? '';
  }

  async upload(file: Blob): Promise<SessionImportResult> {
    return reportSessionRequestFailure(async () => {
      reportFrontendPerformance('session-import-start', { bytes: file.size, type: file.type || 'unknown' });
      if (file.size > MAX_SESSION_IMPORT_BYTES) {
        throw new Error('Session archive exceeds the 128 MiB import limit');
      }
      const response = await this.fetcher(this.endpoint, {
        method: 'POST',
        body: file,
        cache: 'no-store',
        headers: {
          Accept: 'application/json',
          'Content-Type': 'application/octet-stream',
          'X-Float-View-Token': this.token,
        },
      });
      if (!response.ok) {
        reportFrontendPerformance('session-import-response', { status: response.status });
        throw new Error(`Session import failed: HTTP ${response.status}`);
      }
      const result = (await response.json()) as Partial<SessionImportResult>;
      if (typeof result.sessionId !== 'string' || !result.sessionId) {
        throw new Error('Session import returned an invalid session identifier');
      }
      reportFrontendPerformance('session-import-complete', { sessionId: result.sessionId });
      return { sessionId: result.sessionId };
    });
  }
}
