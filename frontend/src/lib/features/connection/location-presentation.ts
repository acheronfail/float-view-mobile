import type { LocationState, SessionMetadata } from '$lib/backend/contract';

export type LocationAction = 'enable' | 'request' | 'settings' | 'none';

export function rideLocationPresentation(location: LocationState | null | undefined) {
  const status = location?.status;
  const off = status === 'disabled';
  const enabled = status === 'authorized';
  const action: LocationAction = off
    ? 'enable'
    : location?.permissionAction === 'openSettings'
      ? 'settings'
      : location?.permissionAction === 'requestPermission'
        ? 'request'
        : 'none';
  return {
    off,
    enabled,
    warning: !off && !enabled,
    label: off ? 'Ride location: off' : enabled ? 'Ride location: enabled' : 'Ride location: unavailable',
    description: off
      ? 'Save your route with this ride. Live readings work with location off.'
      : enabled
        ? 'Route recording is enabled for your rides.'
        : (location?.reason ?? 'Location recording is unavailable. Live readings still work.'),
    action,
    actionLabel: action === 'enable' ? 'Enable' : action === 'settings' ? 'Settings' : 'Allow',
  };
}

export function shouldShowLocationMap(metadata: Pick<SessionMetadata, 'location'> | null | undefined): boolean {
  return metadata?.location.available === true;
}
