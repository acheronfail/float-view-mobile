import { bootstrapSessionExportUrl, type BootstrapDocument } from './bootstrap';

export type SessionExportFormat = 'float-view-archive' | 'csv';

export type SessionExportDownload = {
  fileName: string;
  blob: Blob;
};

export class SessionExportClient {
  private readonly endpoint: URL;
  private readonly token: string;

  constructor(
    bootstrap: BootstrapDocument,
    private readonly development = import.meta.env.DEV,
    base = typeof window === 'undefined' ? 'http://127.0.0.1/' : window.location.href,
    private readonly fetcher: typeof fetch = fetch
  ) {
    this.endpoint = bootstrapSessionExportUrl(bootstrap, base, development);
    this.token = bootstrap.sessionToken ?? '';
  }

  async download(sessionId: string, format: SessionExportFormat): Promise<SessionExportDownload> {
    const response = await this.fetcher(this.endpoint, {
      method: 'POST',
      body: JSON.stringify({ sessionId, format }),
      cache: 'no-store',
      headers: {
        Accept: 'application/octet-stream, text/csv',
        'Content-Type': 'application/json',
        'X-Float-View-Token': this.token,
      },
    });
    if (!response.ok) throw new Error(`Session export download failed: HTTP ${response.status}`);
    return {
      fileName: fileNameFromContentDisposition(
        response.headers.get('Content-Disposition'),
        fallbackFileName(sessionId, format)
      ),
      blob: await response.blob(),
    };
  }
}

export function triggerBrowserDownload(download: SessionExportDownload): void {
  const url = URL.createObjectURL(download.blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = download.fileName;
  link.style.display = 'none';
  document.body.append(link);
  link.click();
  link.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}

function fallbackFileName(sessionId: string, format: SessionExportFormat): string {
  return `float-view-${sessionId}.${format === 'csv' ? 'csv' : 'floatview'}`;
}

function fileNameFromContentDisposition(header: string | null, fallback: string): string {
  if (!header) return fallback;
  const encoded = header.match(/filename\*=UTF-8''([^;]+)/i)?.[1];
  if (encoded) {
    try {
      return decodeURIComponent(encoded);
    } catch {
      return fallback;
    }
  }
  const quoted = header.match(/filename="([^"]+)"/i)?.[1] ?? header.match(/filename=([^;]+)/i)?.[1]?.trim();
  return quoted || fallback;
}
