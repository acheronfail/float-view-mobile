import assert from 'node:assert/strict';
import test from 'node:test';
import { encodeFrame } from '../../../../fixture/frame.ts';
import type { FixtureControl, FixtureEvent } from '../../../../fixture/serial.ts';
import { FixtureActionError, FixtureCoordinator } from './coordinator.server.ts';
import { fixtureCoordinator } from './resolver.server.ts';

test('coordinator attaches, configures atomically, powers, rides, and re-advertises', async () => {
	const fixture = new FakeFixture();
	const coordinator = coordinatorFor(fixture);
	await coordinator.act({ kind: 'attach', path: '/dev/fixture' });
	const configured = await coordinator.act({
		kind: 'configure',
		configuration: { topology: 'express', canId: 42, firmware: '6.05', refloat: '1.2.3' }
	});
	assert.deepEqual(configured.configuration, {
		topology: 'express',
		canId: 42,
		firmware: '6.05',
		refloat: '1.2.3'
	});
	await coordinator.act({ kind: 'power-on' });
	await coordinator.act({ kind: 'start-riding' });
	const revision = coordinator.snapshot().revision;
	await coordinator.act({ kind: 'start-riding' });
	assert.equal(coordinator.snapshot().revision, revision);
	fixture.emit({ kind: 'CONNECTED' });
	fixture.emit({
		kind: 'WRITE',
		bytes: encodeFrame(Uint8Array.of(34, 42, 36, 101, 31))
	});
	await eventually(() => coordinator.snapshot().counters.telemetry === 1);
	assert.equal(coordinator.snapshot().telemetry?.rideState, 'riding');
	fixture.emit({ kind: 'DISCONNECTED' });
	await eventually(() => fixture.commands.filter((value) => value === 'ADVERTISE').length === 2);
	await coordinator.act({ kind: 'stop-riding' });
	assert.equal(coordinator.snapshot().ride, 'idle');
	await coordinator.act({ kind: 'power-off' });
	assert.equal(coordinator.snapshot().ble, 'idle');
	await coordinator.cleanup();
});

test('configuration save is rejected as a whole while active', async () => {
	const fixture = new FakeFixture();
	const coordinator = coordinatorFor(fixture);
	await coordinator.act({ kind: 'attach', path: '/dev/fixture' });
	await coordinator.act({ kind: 'power-on' });
	const before = coordinator.snapshot().configuration;
	await assert.rejects(
		coordinator.act({
			kind: 'configure',
			configuration: { topology: 'direct', canId: 7, firmware: '6.02', refloat: '1.0.0' }
		}),
		FixtureActionError
	);
	assert.deepEqual(coordinator.snapshot().configuration, before);
	await coordinator.cleanup();
});

test('ride requires attached powered-on state and duplicate stop is inert', async () => {
	const fixture = new FakeFixture();
	const coordinator = coordinatorFor(fixture);
	await assert.rejects(coordinator.act({ kind: 'start-riding' }), /Attach/);
	await coordinator.act({ kind: 'attach', path: '/dev/fixture' });
	await assert.rejects(coordinator.act({ kind: 'start-riding' }), /Power on/);
	await coordinator.act({ kind: 'power-on' });
	const revision = coordinator.snapshot().revision;
	await coordinator.act({ kind: 'stop-riding' });
	assert.equal(coordinator.snapshot().revision, revision);
	await coordinator.cleanup();
});

test('serial loss detaches safely and preserves saved configuration', async () => {
	const fixture = new FakeFixture();
	const coordinator = coordinatorFor(fixture);
	await coordinator.act({ kind: 'attach', path: '/dev/fixture' });
	await coordinator.act({
		kind: 'configure',
		configuration: { topology: 'direct', canId: 9, firmware: '6.02', refloat: '1.1.0' }
	});
	await coordinator.act({ kind: 'power-on' });
	fixture.fail(new Error('USB removed'));
	await eventually(() => coordinator.snapshot().serial.status === 'detached');
	const state = coordinator.snapshot();
	assert.equal(state.power, 'off');
	assert.equal(state.ble, 'unavailable');
	assert.deepEqual(state.configuration, {
		topology: 'direct',
		canId: 9,
		firmware: '6.02',
		refloat: '1.1.0'
	});
});

test('ports are bounded and initial attachment failure becomes detached state', async () => {
	const coordinator = new FixtureCoordinator({
		open: async () => {
			throw new Error('Port occupied');
		},
		listPorts: async () =>
			Array.from({ length: 40 }, (_, index) => ({
				path: `/dev/${index}`,
				label: `Fixture ${index}`
			}))
	});
	assert.equal((await coordinator.ports()).length, 32);
	await coordinator.initialise('/dev/missing');
	assert.equal(coordinator.snapshot().serial.status, 'detached');
	assert.match(coordinator.snapshot().serial.error ?? '', /Port occupied/);
});

test('process resolver returns one coordinator across repeated route resolution', () => {
	assert.equal(fixtureCoordinator(), fixtureCoordinator());
});

function coordinatorFor(fixture: FakeFixture) {
	return new FixtureCoordinator({
		open: async () => fixture,
		listPorts: async () => [{ path: '/dev/fixture', label: 'Fixture' }]
	});
}

async function eventually(predicate: () => boolean) {
	for (let index = 0; index < 100; index += 1) {
		if (predicate()) return;
		await new Promise((resolve) => setTimeout(resolve, 2));
	}
	throw new Error('Condition did not become true');
}

class FakeFixture implements FixtureControl {
	readonly commands: string[] = [];
	readonly notifications: Uint8Array[] = [];
	readonly colors: string[] = [];
	private events: FixtureEvent[] = [];
	private waiter?: { resolve: (event: FixtureEvent) => void; reject: (error: Error) => void };

	async command(command: 'RESET' | 'ADVERTISE' | 'DISCONNECT') {
		this.commands.push(command);
		if (command === 'RESET') this.emit({ kind: 'READY' });
	}

	async notify(bytes: Uint8Array) {
		this.notifications.push(bytes);
	}

	async led(rgb: string) {
		this.colors.push(rgb);
	}

	next(timeoutMs = 5_000): Promise<FixtureEvent> {
		const queued = this.events.shift();
		if (queued) return Promise.resolve(queued);
		return new Promise((resolve, reject) => {
			this.waiter = { resolve, reject };
			setTimeout(() => {
				if (this.waiter?.resolve !== resolve) return;
				this.waiter = undefined;
				reject(new Error('Timed out waiting for fake fixture'));
			}, timeoutMs).unref();
		});
	}

	emit(event: FixtureEvent) {
		const waiter = this.waiter;
		if (waiter) {
			this.waiter = undefined;
			waiter.resolve(event);
		} else {
			this.events.push(event);
		}
	}

	fail(error: Error) {
		const waiter = this.waiter;
		if (waiter) {
			this.waiter = undefined;
			waiter.reject(error);
		}
	}

	async close() {
		this.fail(new Error('Fixture serial client closed'));
	}
}
