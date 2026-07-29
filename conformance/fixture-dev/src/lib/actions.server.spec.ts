import assert from 'node:assert/strict';
import test from 'node:test';
import { FixtureCoordinator } from './coordinator/coordinator.server.ts';
import { FIXTURE_DEV_ORIGIN } from './contract.ts';
import { handleActionRequest, parseAction } from './actions.server.ts';

const token = 'test-token';

test('action handler authorizes and atomically applies canonical configuration text', async () => {
	const coordinator = emptyCoordinator();
	const response = await handleActionRequest(
		request({
			kind: 'configure',
			configuration: {
				topology: 'express',
				canId: '42',
				firmware: '6.05',
				refloat: '1.1.0'
			}
		}),
		coordinator,
		token
	);
	assert.equal(response.status, 200);
	const value = (await response.json()) as { configuration: { canId: number } };
	assert.equal(value.configuration.canId, 42);
});

test('action handler rejects origin, token, oversized, and noncanonical CAN values', async () => {
	const coordinator = emptyCoordinator();
	const valid = {
		kind: 'configure',
		configuration: {
			topology: 'express',
			canId: '1',
			firmware: '6.06',
			refloat: '1.2.3'
		}
	};
	assert.equal(
		(
			await handleActionRequest(
				request(valid, { origin: 'http://localhost:11004' }),
				coordinator,
				token
			)
		).status,
		403
	);
	assert.equal(
		(await handleActionRequest(request(valid, { token: 'wrong' }), coordinator, token)).status,
		403
	);
	assert.equal(
		(
			await handleActionRequest(
				request(valid, { contentLength: '4097' }),
				coordinator,
				token
			)
		).status,
		413
	);
	for (const canId of ['01', ' 1', '1 ', '-1', '1.5', '0x2a', '254', '255']) {
		const response = await handleActionRequest(
			request({
				...valid,
				configuration: { ...valid.configuration, canId }
			}),
			coordinator,
			token
		);
		assert.equal(response.status, 400, canId);
	}
});

test('action parser accepts only the bounded high-level action set', () => {
	assert.deepEqual(parseAction({ kind: 'power-on' }), { kind: 'power-on' });
	assert.throws(() => parseAction({ kind: 'motor-control', current: 10 }), /Unknown/);
	assert.throws(() => parseAction({ kind: 'attach', path: '' }), /serial path/);
});

function emptyCoordinator() {
	return new FixtureCoordinator({
		open: async () => {
			throw new Error('not used');
		},
		listPorts: async () => []
	});
}

function request(
	body: unknown,
	options: { origin?: string; token?: string; contentLength?: string } = {}
) {
	const text = JSON.stringify(body);
	return new Request(`${FIXTURE_DEV_ORIGIN}/api/action`, {
		method: 'POST',
		headers: {
			origin: options.origin ?? FIXTURE_DEV_ORIGIN,
			'content-type': 'application/json',
			'content-length': options.contentLength ?? String(text.length),
			'x-fixture-token': options.token ?? token
		},
		body: text
	});
}
