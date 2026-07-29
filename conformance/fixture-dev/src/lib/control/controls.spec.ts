import assert from 'node:assert/strict';
import test from 'node:test';
import type { FixtureSnapshot } from '../contract.ts';
import {
	activateFromKeyboardOrAssistiveClick,
	activateFromPointer
} from './activation.ts';
import {
	canonicalCanId,
	controlAvailability,
	shouldReplaceSnapshot,
	startBoundedPolling
} from './availability.ts';

test('CAN input validation accepts only canonical decimal controller identities', () => {
	assert.equal(canonicalCanId('0'), 0);
	assert.equal(canonicalCanId('42'), 42);
	assert.equal(canonicalCanId('253'), 253);
	for (const value of ['01', ' 1', '1 ', '-1', '1.5', '0x2a', '254', '255', '']) {
		assert.equal(canonicalCanId(value), undefined, value);
	}
});

test('control availability enforces powered-on ride transitions for every topology', () => {
	const snapshot = baseSnapshot();
	assert.equal(controlAvailability(snapshot).startRide, false);
	snapshot.serial = { status: 'attached', path: '/dev/fixture' };
	snapshot.power = 'on';
	snapshot.ble = 'advertising';
	const powered = controlAvailability(snapshot);
	assert.equal(powered.configure, false);
	assert.equal(powered.startRide, true);
	assert.equal(powered.stopRide, false);
	snapshot.ride = 'riding';
	assert.equal(controlAvailability(snapshot).startRide, false);
	assert.equal(controlAvailability(snapshot).stopRide, true);
	snapshot.configuration.topology = 'direct';
	assert.equal(controlAvailability(snapshot).stopRide, true);
});

test('primary pointer and keyboard activation each run exactly once', () => {
	let count = 0;
	const action = () => {
		count += 1;
	};
	activateFromPointer(
		{ isPrimary: true, button: 0, preventDefault() {} } as PointerEvent,
		action
	);
	activateFromKeyboardOrAssistiveClick({ detail: 1 } as MouseEvent, action);
	assert.equal(count, 1);
	activateFromKeyboardOrAssistiveClick({ detail: 0 } as MouseEvent, action);
	assert.equal(count, 2);
	activateFromPointer(
		{ isPrimary: false, button: 0, preventDefault() {} } as PointerEvent,
		action
	);
	assert.equal(count, 2);
});

test('snapshot replacement is monotonic and polling cleanup cancels its timer', () => {
	const current = baseSnapshot();
	assert.equal(shouldReplaceSnapshot(current, { ...current, revision: 1 }), true);
	assert.equal(shouldReplaceSnapshot({ ...current, revision: 2 }, { ...current, revision: 1 }), false);
	let polled = 0;
	let cancelled: unknown;
	const stop = startBoundedPolling(
		() => {
			polled += 1;
		},
		500,
		((callback: () => void, interval: number) => {
			assert.equal(interval, 500);
			callback();
			return 17;
		}) as typeof setInterval,
		((timer: unknown) => {
			cancelled = timer;
		}) as typeof clearInterval
	);
	assert.equal(polled, 1);
	stop();
	assert.equal(cancelled, 17);
});

function baseSnapshot(): FixtureSnapshot {
	return {
		revision: 0,
		serial: { status: 'detached' },
		configuration: {
			topology: 'express',
			canId: 1,
			firmware: '6.06',
			refloat: '1.2.3'
		},
		power: 'off',
		ble: 'unavailable',
		ride: 'idle',
		counters: { writes: 0, responses: 0, telemetry: 0, dropped: 0 },
		diagnostics: []
	};
}
