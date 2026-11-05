import { describe, expect, it } from 'vitest';
import {
  frontendBuildInfo,
  backendProxyOrigin,
  DEVELOPMENT_PORT,
  MOCK_BACKEND_ORIGIN,
  selectedBackendOrigin,
} from '../../../vite.config';

describe('Vite backend selection', () => {
  it('uses the stable mock and Vite ports by default', () => {
    expect(DEVELOPMENT_PORT).toBe(11002);
    expect(MOCK_BACKEND_ORIGIN).toBe('http://127.0.0.1:11003');
    expect(selectedBackendOrigin({})).toBe(MOCK_BACKEND_ORIGIN);
  });

  it('accepts only an explicit loopback HTTP origin', () => {
    expect(selectedBackendOrigin({ FLOAT_VIEW_DEV_BACKEND_ORIGIN: 'http://127.0.0.1:11010' })).toBe(
      'http://127.0.0.1:11010'
    );
    expect(() => selectedBackendOrigin({ FLOAT_VIEW_DEV_BACKEND_ORIGIN: 'http://192.168.1.2:11010' })).toThrow(
      'must be an http://127.0.0.1'
    );
  });

  it('forwards native auth requests with the backend origin while preserving mock auth', () => {
    expect(backendProxyOrigin('http://127.0.0.1:11010', 'http://127.0.0.1:11002')).toBe('http://127.0.0.1:11010');
    expect(backendProxyOrigin(MOCK_BACKEND_ORIGIN, 'http://127.0.0.1:11002')).toBe('http://127.0.0.1:11002');
    expect(backendProxyOrigin('http://127.0.0.1:11010', undefined)).toBeUndefined();
  });
});

// Classification uses the same validated version grammar as release tooling.
describe('About build metadata', () => {
  it.each([
    [null, true],
    ['v1.2.3', false],
    ['v1.2.3-rc.2', false],
    ['v1.2.4-nightly.12.gabcdef012345', true],
  ])('sets revision visibility for %s', (version, showRevision) => {
    expect(frontendBuildInfo(version, 'abcdef012345')).toEqual({ version, revision: 'abcdef012345', showRevision });
  });
  it('rejects malformed versions instead of treating them as releases', () => {
    expect(() => frontendBuildInfo('v1.2.3-rc.invalid', 'abcdef012345')).toThrow();
  });
});
