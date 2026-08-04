import { describe, expect, it } from 'vitest';
import type { SessionMetadata } from '$lib/backend/contract';
import { locationAction, locationRequiresAction, shouldShowLocationMap } from './location-presentation';

describe('location presentation state', () => {
  it('activates the live warning for disabled and unresolved permissions', () => {
    expect(
      locationRequiresAction({
        status: 'disabled',
        permissionAction: 'requestPermission',
        backgroundCapture: 'unavailable',
        reason: null,
      })
    ).toBe(true);
    expect(
      locationRequiresAction({
        status: 'authorized',
        permissionAction: null,
        backgroundCapture: 'available',
        reason: null,
      })
    ).toBe(false);
    expect(
      locationAction({
        status: 'disabled',
        permissionAction: 'requestPermission',
        backgroundCapture: 'unavailable',
        reason: null,
      })
    ).toBe('enable');
    expect(
      locationAction({
        status: 'denied',
        permissionAction: 'openSettings',
        backgroundCapture: 'unavailable',
        reason: 'Denied',
      })
    ).toBe('settings');
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
