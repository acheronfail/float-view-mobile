import type { BluetoothState, PermissionRow } from '$lib/backend/contract';

export function discoveryLocationPermission(rows: PermissionRow[] = []) {
  return rows.find(
    (row) =>
      row.permission === 'location' &&
      !row.optional &&
      (row.status !== 'granted' || row.accuracy !== 'precise' || row.servicesEnabled !== true)
  );
}

export function discoveryNotice(
  bluetooth: BluetoothState | undefined,
  permissionPending = false,
  location?: PermissionRow
) {
  if (!bluetooth) return null;
  if (location && (bluetooth.scanPermission !== 'not_determined' || bluetooth.connectPermission === 'not_required')) {
    if (location.pending || permissionPending)
      return {
        title: 'Waiting for location access',
        detail:
          'Respond to your device’s permission prompt. Choose precise location and allow access while using Float View to find nearby boards.',
      };
    if (location.servicesEnabled === false && ['granted', 'limited'].includes(location.status))
      return {
        title: 'Location is off',
        detail:
          'Android needs Location turned on to find nearby Bluetooth boards. Open settings and turn on Location. Scanning will resume automatically.',
      };
    return {
      title: 'Location access is needed to find boards',
      detail:
        location.status === 'restricted'
          ? 'Location access is restricted by your device. Check your device’s restrictions to allow board discovery.'
          : location.status === 'unavailable'
            ? 'Waiting for your device to report location access before scanning for boards.'
            : 'Android needs precise location access to find nearby Bluetooth boards. Allow access while using Float View and enable Precise location. Ride route recording stays optional.',
    };
  }
  if (permissionPending) {
    return {
      title: 'Waiting for Bluetooth access',
      detail: 'Respond to your device’s permission prompt so Float View can find your board.',
    };
  }
  if (bluetooth.scanPermission === 'not_determined') {
    return {
      title: 'Bluetooth access is needed',
      detail: 'Allow Float View to use Bluetooth when prompted. Scanning will start after access is granted.',
    };
  }
  if (bluetooth.radio === 'powered_off') {
    return {
      title: 'Bluetooth is off',
      detail:
        'Turn on Bluetooth in your device’s Settings to find and connect to your board. Scanning will resume automatically when Bluetooth is on.',
    };
  }
  if (bluetooth.radio === 'unsupported') {
    return { title: 'Bluetooth is unavailable', detail: 'This device cannot discover or connect to Bluetooth boards.' };
  }
  if (bluetooth.scanPermission === 'denied' || bluetooth.scanPermission === 'restricted') {
    return {
      title: 'Bluetooth access is needed',
      detail:
        bluetooth.scanPermission === 'restricted'
          ? 'Bluetooth access is restricted by your device. Check your device’s restrictions to allow access.'
          : 'Allow Float View to use Bluetooth in your device’s Settings to find and connect to your board.',
    };
  }
  if (bluetooth.radio === 'resetting') {
    return {
      title: 'Bluetooth is restarting',
      detail: 'Waiting for Bluetooth. Scanning will resume automatically when it is ready.',
    };
  }
  if (bluetooth.radio === 'unknown' || bluetooth.scanPermission === 'unknown') {
    return {
      title: 'Checking Bluetooth',
      detail: 'Waiting for your device to report Bluetooth availability. Scanning will start when it is ready.',
    };
  }
  return null;
}
