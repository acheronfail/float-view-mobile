import assert from 'node:assert/strict';
import type { AppState, BackendMethod } from '../../frontend/src/lib/backend/contract.js';
import { ConformanceClient, ConformanceError } from './client.js';

export interface Target {
	name: string;
	start(): Promise<{ url: string; origin: string }>;
	stop(): Promise<void>;
}

export async function runConformance(target: Target) {
	const endpoint = await target.start();
	const client = await ConformanceClient.open(endpoint.url, endpoint.origin);
	try {
		const initial = await client.request('state.get', {});
		assertState(initial);
		await expectError(
			(client as unknown as { request(method: string, params: unknown): Promise<unknown> }).request(
				'unknown.method',
				{}
			),
			'invalid_request'
		);

		const permission = await client.request('bluetooth.requestPermission', {});
		assert.equal(typeof permission.radio, 'string');
		await client.request('bluetooth.scan', { timeoutMs: 100 });
		const scanned = await client.request('state.get', {});
		assert.ok(scanned.revision >= initial.revision);

		await exerciseProductMethods(client, scanned);
		return { target: target.name, revision: (await client.request('state.get', {})).revision };
	} finally {
		client.close();
		await target.stop();
	}
}

async function exerciseProductMethods(client: ConformanceClient, state: AppState) {
	const device = state.discoveredDevices[0];
	if (device) {
		await client.request('device.connect', { deviceId: device.deviceId });
		const connected = await client.request('state.get', {});
		assert.ok(connected.activeConnection);
		await expectError(
			client.request('device.connect', { deviceId: device.deviceId }),
			'invalid_state'
		);
		await client.request('device.disconnect', {});
	}

	const profileDevice = state.savedDevices[0];
	if (profileDevice) {
		const profile = await client.request('boardProfile.get', {
			savedDeviceId: profileDevice.savedDeviceId
		});
		await client.request('boardProfile.update', {
			savedDeviceId: profile.savedDeviceId,
			expectedRevision: profile.revision,
			patch: { seriesCount: 18, parallelCount: 2, motor: 'hypercore_xr' }
		});
		await expectError(
			client.request('boardProfile.update', {
				savedDeviceId: profile.savedDeviceId,
				expectedRevision: profile.revision,
				patch: {}
			}),
			'conflict'
		);
	}

	const current = await client.request('state.get', {});
	await client.request('settings.update', {
		expectedRevision: current.settings.revision,
		patch: { automaticSessionLinkingEnabled: !current.settings.automaticSessionLinkingEnabled }
	});

	const session = current.recentSessions[0];
	if (session) {
		const detail = await client.request('session.get', { sessionId: session.sessionId, maxPoints: 100 });
		assert.equal(detail.summary.sessionId, session.sessionId);
		assert.ok(detail.series.length <= 100);
		await client.request('session.getDiagnostics', {
			sessionId: session.sessionId,
			centerElapsedMs: 0,
			beforeMs: 1_000,
			afterMs: 1_000
		});
		await client.request('session.export', { sessionId: session.sessionId, format: 'csv' });
	}
}

function assertState(state: AppState) {
	assert.ok(Number.isSafeInteger(state.revision));
	assert.ok(Array.isArray(state.discoveredDevices));
	assert.ok(Array.isArray(state.savedDevices));
	assert.ok(Array.isArray(state.recentSessions));
	assert.ok(Array.isArray(state.sessionGroups));
	assert.ok(state.discoveredDevices.length <= 50);
	assert.ok(state.savedDevices.length <= 50);
	assert.ok(state.recentSessions.length <= 50);
}

async function expectError(promise: Promise<unknown>, code: string) {
	await assert.rejects(promise, (error: unknown) => {
		return error instanceof ConformanceError && error.error.code === code;
	});
}

export const mandatoryMethods: BackendMethod[] = [
	'state.get',
	'bluetooth.requestPermission',
	'bluetooth.scan',
	'device.connect',
	'device.disconnect',
	'settings.update',
	'boardProfile.get',
	'boardProfile.update',
	'session.get',
	'session.getDiagnostics',
	'session.export',
	'session.delete',
	'sessionGroup.unlink'
];
