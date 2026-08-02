import { describe, expect, it } from 'vitest';
import { sessionExportLabel } from './session-export';

describe('session export labels', () => {
  it.each([
    ['ios', 'csv', 'Share CSV'],
    ['android', 'float-view-archive', 'Share Archive'],
    ['macos', 'csv', 'Save CSV'],
    ['web', 'float-view-archive', 'Download Archive'],
  ] as const)('uses the %s action for %s exports', (platform, format, label) => {
    expect(sessionExportLabel(platform, format)).toBe(label);
  });
});
