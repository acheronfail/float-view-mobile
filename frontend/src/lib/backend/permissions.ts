import type { PermissionRow } from './contract.ts';

/** Reject malformed native inventories instead of inventing labels or actions. */
export function validatePermissions(value: unknown): asserts value is PermissionRow[] {
  if (!Array.isArray(value) || value.length < 2 || value.length > 3) throw new Error('Invalid permission inventory');
  const ids = new Set<string>();
  const grants = [null, 'unknown', 'notDetermined', 'granted', 'denied', 'restricted', 'notRequired'];
  for (const row of value) {
    if (
      !row ||
      typeof row !== 'object' ||
      !['bluetooth', 'location', 'backgroundLocation'].includes(row.permission) ||
      ids.has(row.permission) ||
      !['notDetermined', 'granted', 'limited', 'denied', 'restricted', 'unavailable'].includes(row.status) ||
      ![null, 'requestPermission', 'openSettings'].includes(row.action) ||
      ![
        'none',
        'checking',
        'restricted',
        'servicesDisabled',
        'unsupportedHardware',
        'foregroundRequired',
        'rationale',
        'appSettings',
        'iosSettings',
        'macBluetooth',
        'macLocation',
        'backgroundSettings',
        'alwaysAccess',
        'fineLocationForScan',
      ].includes(row.guidance) ||
      typeof row.pending !== 'boolean' ||
      typeof row.optional !== 'boolean' ||
      ![null, true, false].includes(row.servicesEnabled) ||
      !grants.includes(row.scan) ||
      !grants.includes(row.connect) ||
      !['unknown', 'approximate', 'precise'].includes(row.accuracy) ||
      !['unknown', 'whileInUse', 'always'].includes(row.scope) ||
      !(
        row.optionLabel === null ||
        (typeof row.optionLabel === 'string' &&
          [...row.optionLabel].length <= 120 &&
          // Native labels must not contain control characters.
          // eslint-disable-next-line no-control-regex
          !/[\u0000-\u001f\u007f]/u.test(row.optionLabel))
      )
    )
      throw new Error('Invalid permission row');
    ids.add(row.permission);
  }
  if (!ids.has('location')) throw new Error('Missing location permission');
}
