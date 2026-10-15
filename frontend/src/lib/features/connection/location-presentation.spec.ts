import { describe, expect, it } from 'vitest';
import type { SessionMetadata } from '$lib/backend/contract';
import { rideLocationPresentation, shouldShowLocationMap } from './location-presentation';

describe('location presentation state', () => {
  it.each([
    ['disabled', 'requestPermission', 'enable', false],
    ['authorized', null, 'none', false],
    ['notDetermined', 'requestPermission', 'request', true],
    ['denied', 'openSettings', 'settings', true],
    ['foregroundOnly', 'openSettings', 'settings', true],
    ['restricted', null, 'none', true],
    ['unavailable', null, 'none', true],
  ] as const)(
    'presents %s with an actionable recovery only when supported',
    (status, permissionAction, action, warning) => {
      expect(
        rideLocationPresentation({ status, permissionAction, backgroundCapture: 'unavailable', reason: null })
      ).toMatchObject({ action, warning, off: status === 'disabled', enabled: status === 'authorized' });
    }
  );

  it('retains the native explanation for blocked recording', () => {
    expect(
      rideLocationPresentation({
        status: 'denied',
        permissionAction: 'openSettings',
        backgroundCapture: 'unavailable',
        reason: 'Enable location in Settings.',
      }).description
    ).toBe('Enable location in Settings.');
    expect(rideLocationPresentation(undefined).action).toBe('none');
  });

  it('omits the map when metadata has no accepted location track', () => {
    expect(shouldShowLocationMap(undefined)).toBe(false);
    expect(
      shouldShowLocationMap({
        location: { available: false, count: 0, lastSequence: 0, lastElapsedMs: 0, status: 'disabled' },
      } satisfies Pick<SessionMetadata, 'location'>)
    ).toBe(false);
  });
});
