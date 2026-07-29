import { describe, expect, it } from 'vitest';
import type { ActiveConnection } from '$lib/backend/contract';
import { readingsAreDimmed, shouldConfirmDisconnect } from './live';

const connection = (state: ActiveConnection['state']): ActiveConnection => ({
	deviceId: 'board',
	name: 'Board',
	displayName: 'Board',
	state,
	firmware: null,
	savedDeviceId: null,
	activeSession: null,
	latestSample: null
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
});
