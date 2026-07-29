import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { ActiveConnection } from '$lib/backend/contract';
import ControllerSelection from './ControllerSelection.svelte';

const connection: ActiveConnection = {
	deviceId: 'board',
	name: 'Board',
	displayName: 'Board',
	state: 'selecting_controller',
	savedDeviceId: 'saved-board',
	controllerConnection: null,
	controllerCandidates: [
		{
			canId: 42,
			hardwareName: 'THOR300',
			firmwareName: 'Refloat',
			vescVersion: '6.06',
			vescSupported: true
		},
		{
			canId: 7,
			hardwareName: 'Old VESC',
			firmwareName: null,
			vescVersion: '5.03',
			vescSupported: false
		}
	],
	connectionIssue: null,
	firmware: null,
	latestSample: null,
	activeSession: null
};

describe('controller selection prompt', () => {
	it('sorts candidates by CAN ID and leaves unsupported controllers visible but disabled', () => {
		const { body } = render(ControllerSelection, {
			props: { connection, ondisconnect: async () => undefined }
		});
		const text = body.replace(/\s+/g, ' ');

		expect(text).toContain('Choose a controller (2 found)');
		expect(text.indexOf('CAN 7')).toBeLessThan(text.indexOf('CAN 42'));
		expect(text).toContain('Old VESC · Unknown firmware · VESC 5.03');
		expect(text).toContain('THOR300 · Refloat · VESC 6.06');
		expect(body.match(/ disabled(?:>| )/g)).toHaveLength(1);
		expect(text).toContain('Unsupported');
		expect(text).toContain('Supported');
		expect(text).toContain('Disconnect');
	});
});
