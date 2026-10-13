import type { BluetoothState } from '$lib/backend/contract';

export function discoveryNotice(bluetooth: BluetoothState | undefined) {
  if (!bluetooth) return null;
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
  return null;
}
