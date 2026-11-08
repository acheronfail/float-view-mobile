import { describe, expect, it } from 'vitest';
import { backendErrorMessage } from './error-message';
import type { BackendError } from './contract';

describe('backend error presentation', () => {
  it.each<BackendError>([
    { code: 'invalid_request', message: 'Request is invalid' },
    { code: 'invalid_state', message: 'Backend state does not permit the operation' },
    { code: 'permission_required', message: 'Permission is required' },
    { code: 'not_found', message: 'Resource was not found' },
    { code: 'conflict', message: 'Resource revision changed' },
    { code: 'unavailable', message: 'Backend service is unavailable' },
    { code: 'internal', message: 'Backend operation failed' },
  ])('gives a recovery action for generic $code errors', (error) => {
    const message = backendErrorMessage(error);
    expect(message).not.toBe(error.message);
    expect(message).not.toMatch(/Backend|Resource|Request is invalid/);
    expect(message.length).toBeGreaterThan(40);
  });

  it('identifies the attempted operation and preserves useful custom guidance', () => {
    const generic: BackendError = { code: 'invalid_state', message: 'Backend state does not permit the operation' };
    expect(backendErrorMessage(generic, 'bluetooth.scan')).toContain('scan for boards');
    expect(backendErrorMessage(generic, 'device.connect')).toContain('connect to the board');
    expect(backendErrorMessage(generic, 'refloat.configuration.read')).toContain('read the configuration');
    expect(backendErrorMessage(generic, 'refloat.configuration.write')).toContain('read current values');
    const specific: BackendError = { code: 'invalid_state', message: 'Turn off flywheel mode before writing.' };
    expect(backendErrorMessage(specific, 'refloat.configuration.write')).toBe(specific.message);
  });
});
