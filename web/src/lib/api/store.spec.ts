import { describe, expect, it } from 'vitest';
import type { BluetoothState } from '../../../../api/generated/typescript/models.js';
import { permissionOutcome } from './store.svelte';

function state(scanPermission: BluetoothState['scanPermission']): BluetoothState {
	return {
		radio: 'powered_on',
		scanPermission,
		connectPermission: 'granted'
	};
}

describe('permissionOutcome', () => {
	it.each([
		['granted', 'ready'],
		['not_required', 'ready'],
		['not_determined', 'pending'],
		['unknown', 'pending'],
		['denied', 'denied'],
		['restricted', 'restricted']
	] as const)('maps %s to %s', (permission, outcome) => {
		expect(permissionOutcome(state(permission))).toBe(outcome);
	});
});
