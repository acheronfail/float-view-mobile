import { describe, expect, it } from 'vitest';
import type { ActiveConnection } from '$lib/backend/contract';
import {
	controllerRouteDescription,
	firmwareAccessDescription,
	readingsAreDimmed,
	shouldConfirmDisconnect,
	sortedControllerCandidates
} from './live';

const connection = (state: ActiveConnection['state']): ActiveConnection => ({
	deviceId: 'board',
	name: 'Board',
	displayName: 'Board',
	state,
	firmware: null,
	savedDeviceId: null,
	controllerConnection: null,
	controllerCandidates: [],
	connectionIssue: null,
	activeSession: null,
	latestSample: null,
	currentLimits: null
});

describe('live board behavior', () => {
	it('dims retained readings for every state except ready', () => {
		expect(readingsAreDimmed(connection('ready'))).toBe(false);
		expect(readingsAreDimmed(connection('reconnecting'))).toBe(true);
		expect(readingsAreDimmed(connection('disconnecting'))).toBe(true);
	});

	it('skips confirmation only for an exactly idle latest sample', () => {
		expect(shouldConfirmDisconnect('idle')).toBe(false);
		expect(shouldConfirmDisconnect('riding')).toBe(true);
		expect(shouldConfirmDisconnect('unknown')).toBe(true);
		expect(shouldConfirmDisconnect(undefined)).toBe(true);
	});

	it('describes direct and CAN routes without exposing private identity', () => {
		expect(controllerRouteDescription({ kind: 'direct' })).toBe('Direct Bluetooth');
		expect(controllerRouteDescription({ kind: 'can', canId: 42 })).toBe('CAN 42');
		expect(controllerRouteDescription(null)).toBe('Detecting…');
	});

	it('labels audited test firmware as read-only', () => {
		expect(
			firmwareAccessDescription({
				vescVersion: '7.00',
				packageName: 'refloat',
				packageVersion: '1.2.3',
				hardwareName: 'THOR300',
				accessMode: 'readOnly',
				supported: true
			})
		).toBe('Read-only (test firmware)');
	});

	it('sorts supported and unsupported candidates by CAN ID without hiding either', () => {
		const candidates = sortedControllerCandidates([
			{
				canId: 42,
				hardwareName: 'THOR300',
				firmwareName: 'Refloat',
				vescVersion: '6.06',
				vescSupported: true
			},
			{
				canId: 7,
				hardwareName: 'Older controller',
				firmwareName: null,
				vescVersion: '5.03',
				vescSupported: false
			}
		]);
		expect(candidates.map(({ canId, vescSupported }) => [canId, vescSupported])).toEqual([
			[7, false],
			[42, true]
		]);
	});
});
