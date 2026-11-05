import { reportSessionRequestFailure } from './request-failures';
import { bootstrapSessionImportUrl, type BootstrapDocument } from './bootstrap';
import { MAX_APP_TRANSFER_BYTES, MAX_SESSION_IMPORT_BYTES, type AppImportSummary } from './contract';
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

  async uploadApp(file: Blob, applySettings: boolean): Promise<AppImportSummary> {
    return reportSessionRequestFailure(async () => {
      if (file.size > MAX_APP_TRANSFER_BYTES) throw new Error('App export exceeds the 2 GiB import limit');
      const response = await this.fetcher(new URL('app-import', this.endpoint), {
        method: 'POST',
        body: file,
        cache: 'no-store',
        headers: {
          Accept: 'application/json',
          'Content-Type': 'application/octet-stream',
          'X-Float-View-Token': this.token,
          'X-Float-View-Import-Settings': String(applySettings),
        },
      });
      if (!response.ok)
        throw new Error(`App import failed: HTTP ${response.status}. Your existing data has been preserved.`);
      const result = (await response.json()) as AppImportSummary;
      for (const key of [
        'boardsAdded',
        'boardsSkipped',
        'sessionsAdded',
        'sessionsSkipped',
        'tunesAdded',
        'tunesSkipped',
      ] as const) {
        if (!Number.isSafeInteger(result[key]) || result[key] < 0) throw new Error('Invalid app import summary');
      }
      if (typeof result.settingsImported !== 'boolean') throw new Error('Invalid app import summary');
      return result;
    });
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
