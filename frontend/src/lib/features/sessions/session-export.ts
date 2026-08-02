import type { FrontendPlatform } from '$lib/backend/bootstrap';
import type { SessionExportFormat } from '$lib/backend/session-export-client';

export function sessionExportLabel(platform: FrontendPlatform, format: SessionExportFormat): string {
  const action = platform === 'ios' || platform === 'android' ? 'Share' : platform === 'macos' ? 'Save' : 'Download';
  return `${action} ${format === 'csv' ? 'CSV' : 'Archive'}`;
}
