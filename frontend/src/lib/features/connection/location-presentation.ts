import type { LocationState, SessionMetadata } from '$lib/backend/contract';

export type LocationAction = 'enable' | 'request' | 'settings' | 'none';

export function locationRequiresAction(location: LocationState | null | undefined): boolean {
  return location !== null && location !== undefined && location.status !== 'authorized';
}

export function locationAction(location: LocationState | null | undefined): LocationAction {
  if (!location) return 'none';
  if (location.status === 'disabled') return 'enable';
  return location.permissionAction === 'openSettings' ? 'settings' : 'request';
}

export function shouldShowLocationMap(metadata: Pick<SessionMetadata, 'location'> | null | undefined): boolean {
  return metadata?.location.available === true;
}
