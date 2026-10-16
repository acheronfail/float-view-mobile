import type { PermissionRow, LocationState } from '../src/lib/backend/contract.ts';

export function permissionInventory(
  platform: 'ios' | 'macos' | 'android29' | 'android30' | 'android31' | 'android33' | 'android36'
): PermissionRow[] {
  const location: PermissionRow = {
    permission: 'location',
    status: 'notDetermined',
    action: 'requestPermission',
    guidance: 'none',
    pending: false,
    optional: true,
    servicesEnabled: true,
    scan: null,
    connect: null,
    accuracy: 'unknown',
    scope: 'unknown',
    optionLabel: null,
  };
  if (platform.startsWith('android')) {
    location.optional = false;
    location.guidance = 'fineLocationForScan';
  }
  const bluetooth: PermissionRow = {
    ...location,
    permission: 'bluetooth',
    guidance: 'none',
    optional: false,
    status: 'granted',
    action: null,
    scan: 'granted',
    connect: 'granted',
  };
  const background: PermissionRow = {
    ...location,
    permission: 'backgroundLocation',
    optional: true,
    action: null,
    guidance: 'foregroundRequired',
  };
  const notifications: PermissionRow = {
    ...location,
    permission: 'notifications',
    optional: true,
    servicesEnabled: null,
    status: platform === 'android33' || platform === 'android36' ? 'notDetermined' : 'granted',
    action: platform === 'android33' || platform === 'android36' ? 'requestPermission' : 'openSettings',
    guidance: platform === 'android33' || platform === 'android36' ? 'none' : 'notificationSettings',
  };
  const liveUpdates: PermissionRow = {
    ...notifications,
    permission: 'liveUpdates',
    status: 'denied',
    action: null,
    guidance: 'notificationsRequired',
  };
  return platform.startsWith('android')
    ? [
        ...(!['android29', 'android30'].includes(platform) ? [bluetooth] : []),
        location,
        background,
        notifications,
        ...(platform === 'android36' ? [liveUpdates] : []),
      ]
    : [bluetooth, location];
}

export function locationSummary(rows: PermissionRow[], enabled: boolean): LocationState {
  const location = rows.find((row) => row.permission === 'location')!;
  const background = rows.find((row) => row.permission === 'backgroundLocation');
  const granted = ['granted', 'limited'].includes(location.status);
  const available = granted && location.servicesEnabled !== false;
  return {
    status: !enabled
      ? 'disabled'
      : !available
        ? location.status === 'limited' || location.status === 'granted'
          ? 'unavailable'
          : location.status
        : location.scope === 'always'
          ? 'authorized'
          : 'foregroundOnly',
    permissionAction: location.action ?? background?.action ?? null,
    backgroundCapture: available ? (location.scope === 'always' ? 'available' : 'foregroundOnly') : 'unavailable',
    reason: null,
  };
}
