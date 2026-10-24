import assert from 'node:assert/strict';
import test from 'node:test';
import { encodeFrame } from '../../../../fixture/frame.ts';
import type { FixtureControl, FixtureEvent } from '../../../../fixture/serial.ts';
import { FixtureActionError, FixtureCoordinator } from './coordinator.server.ts';
import { fixtureCoordinator } from './resolver.server.ts';
import type { FixtureConfiguration } from '../contract.ts';

test('coordinator attaches, configures atomically, powers, rides, and re-advertises', async () => {
  const fixture = new FakeFixture();
  const logs: string[] = [];
  const coordinator = coordinatorFor(fixture, (_level, message) => logs.push(message));
  await coordinator.act({ kind: 'attach', path: '/dev/fixture' });
  const configured = await coordinator.act({
    kind: 'configure',
    configuration: configuration({
      topology: 'express',
      canId: 42,
      firmware: '6.05',
      refloat: '1.2.3',
      bmsTopology: 'can',
      bmsCanId: 43,
    }),
  });
  assert.deepEqual(configured.configuration, {
    topology: 'express',
    canId: 42,
    firmware: '6.05',
    refloat: '1.2.3',
    bmsTopology: 'can',
    bmsCanId: 43,
    bmsVariant: 'extended',
    bmsFailureMode: 'none',
    bmsCellCount: 15,
    bmsTemperatureCount: 3,
    bmsStateOfCharge: 0.75,
    bmsInputCurrentA: 0,
  });
  await coordinator.act({ kind: 'power-on' });
  await coordinator.act({ kind: 'start-riding' });
  const revision = coordinator.snapshot().revision;
  await coordinator.act({ kind: 'start-riding' });
  assert.equal(coordinator.snapshot().revision, revision);
  fixture.emit({ kind: 'CONNECTED' });
  fixture.emit({ kind: 'WRITE', bytes: encodeFrame(Uint8Array.of(0)) });
  await eventually(() => coordinator.snapshot().counters.responses === 1);
  assert.equal(coordinator.snapshot().lastRequest?.kind, 'transport-firmware');
  assert.ok((coordinator.snapshot().lastRequest?.fragments ?? 0) > 1);
  fixture.emit({ kind: 'WRITE', bytes: encodeFrame(Uint8Array.of(34, 42, 14)) });
  await eventually(() => coordinator.snapshot().counters.responses === 2);
  assert.equal(coordinator.snapshot().lastRequest?.kind, 'motor-configuration');
  assert.deepEqual(coordinator.snapshot().lastAttempt, {
    command: 14,
    canId: 42,
    stage: 'notify',
    outcome: 'responded',
  });
  assert.ok(logs.includes('request command=14 route=CAN:42'));
  fixture.emit({ kind: 'WRITE', bytes: encodeFrame(Uint8Array.of(34, 43, 96)) });
  await eventually(() => coordinator.snapshot().counters.bms === 1);
  assert.equal(coordinator.snapshot().lastRequest?.kind, 'bms-values');
  fixture.emit({
    kind: 'WRITE',
    bytes: encodeFrame(Uint8Array.of(34, 42, 36, 101, 31)),
  });
  await eventually(() => coordinator.snapshot().counters.telemetry === 1);
  assert.equal(coordinator.snapshot().telemetry?.rideState, 'riding');
  assert.ok((coordinator.snapshot().lastRequest?.fragments ?? 0) > 1);
  assert.equal(
    fixture.notifications.every((value) => value.byteLength <= 20),
    true
  );
  fixture.emit({ kind: 'DISCONNECTED' });
  await eventually(() => fixture.commands.filter((value) => value === 'ADVERTISE').length === 2);
  await coordinator.act({ kind: 'stop-riding' });
  assert.equal(coordinator.snapshot().ride, 'idle');
  await coordinator.act({ kind: 'power-off' });
  assert.equal(coordinator.snapshot().ble, 'idle');
  await coordinator.cleanup();
});

test('semantic scenes, pins, freeze, resume, and reset remain manual and shareable', async () => {
  const coordinator = coordinatorFor(new FakeFixture());
  await coordinator.act({ kind: 'set-scene', scene: 'acceleration' });
  const accelerated = coordinator.snapshot().telemetry!;
  assert.equal(accelerated.packageState, 'running');
  assert.ok(accelerated.requestedBalanceCurrentA > accelerated.motorCurrentA);
  await coordinator.act({ kind: 'pin-field', field: 'controllerTempC', value: 77 });
  await coordinator.act({ kind: 'freeze' });
  assert.equal(coordinator.snapshot().frozen, true);
  assert.equal(coordinator.snapshot().pinnedFields.controllerTempC, 77);
  assert.equal(coordinator.snapshot().scenario.overrides.controllerTempC, 77);
  await coordinator.act({ kind: 'resume' });
  await coordinator.act({ kind: 'reset-overrides' });
  assert.equal(coordinator.snapshot().frozen, false);
  assert.deepEqual(coordinator.snapshot().pinnedFields, {});
});

test('active move and replacement events serialize disconnect, mutation, and re-advertise', async () => {
  const fixture = new FakeFixture();
  const coordinator = coordinatorFor(fixture);
  await coordinator.act({ kind: 'attach', path: '/dev/fixture' });
  await coordinator.act({ kind: 'power-on' });
  await coordinator.act({ kind: 'move-controller', alias: 'primary', canId: 42 });
  assert.equal(coordinator.snapshot().scenario.controllers[0]?.canId, 42);
  assert.deepEqual(fixture.commands.slice(-2), ['DISCONNECT', 'ADVERTISE']);
  await coordinator.act({ kind: 'replace-controller', alias: 'primary' });
  assert.match(coordinator.snapshot().scenario.controllers[0]?.alias ?? '', /^replacement-/);
  assert.equal(JSON.stringify(coordinator.snapshot()).toLowerCase().includes('uuid'), false);
  assert.deepEqual(fixture.commands.slice(-2), ['DISCONNECT', 'ADVERTISE']);
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
      configuration: configuration({ topology: 'direct', canId: 7, firmware: '6.02', refloat: '1.0.0' }),
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
  const logs: string[] = [];
  const coordinator = coordinatorFor(fixture, (level, message) => logs.push(`${level}:${message}`));
  await coordinator.act({ kind: 'attach', path: '/dev/fixture' });
  const saved = await coordinator.act({
    kind: 'configure',
    configuration: configuration({ topology: 'direct', canId: 9, firmware: '6.02', refloat: '1.1.0' }),
  });
  await coordinator.act({ kind: 'power-on' });
  fixture.fail(new Error('USB removed'));
  await eventually(() => coordinator.snapshot().serial.status === 'detached');
  const state = coordinator.snapshot();
  assert.equal(state.power, 'off');
  assert.equal(state.ble, 'unavailable');
  assert.match(state.serial.error ?? '', /USB removed/);
  assert.ok(state.diagnostics.some((value) => value.includes('Serial detached: USB removed')));
  assert.ok(logs.includes('error:serial detached: USB removed'));
  assert.deepEqual(state.configuration, saved.configuration);
});

test('unsupported protocol requests are visible and do not detach serial', async () => {
  const fixture = new FakeFixture();
  const logs: string[] = [];
  const coordinator = coordinatorFor(fixture, (level, message) => logs.push(`${level}:${message}`));
  await coordinator.act({ kind: 'attach', path: '/dev/fixture' });
  await coordinator.act({ kind: 'power-on' });
  fixture.emit({ kind: 'CONNECTED' });
  fixture.emit({ kind: 'WRITE', bytes: encodeFrame(Uint8Array.of(34, 1, 99)) });
  await eventually(() => coordinator.snapshot().counters.dropped === 1);
  const dropped = coordinator.snapshot();
  assert.equal(dropped.serial.status, 'attached');
  assert.equal(dropped.power, 'on');
  assert.deepEqual(dropped.lastAttempt, {
    command: 99,
    canId: 1,
    stage: 'simulate',
    outcome: 'dropped',
    error: 'Unsafe or unsupported forwarded command 99',
  });
  assert.ok(dropped.diagnostics.some((value) => value.includes('command=99 route=CAN:1')));
  assert.ok(logs.some((value) => value.includes('warn:Protocol request dropped during simulate')));

  fixture.emit({ kind: 'WRITE', bytes: encodeFrame(Uint8Array.of(0)) });
  await eventually(() => coordinator.snapshot().counters.responses === 1);
  assert.equal(coordinator.snapshot().serial.status, 'attached');
  await coordinator.cleanup();
});

test('configuration rejection logs retain the route and specific sanitized reason', async () => {
  for (const topology of ['direct', 'express'] as const) {
    const fixture = new FakeFixture();
    const logs: string[] = [];
    const coordinator = coordinatorFor(fixture, (_level, message) => logs.push(message));
    await coordinator.act({ kind: 'attach', path: '/dev/fixture' });
    await coordinator.act({ kind: 'configure', configuration: configuration({ topology }) });
    await coordinator.act({ kind: 'power-on' });
    fixture.emit({ kind: 'CONNECTED' });
    const route = (payload: number[]) =>
      encodeFrame(Uint8Array.from(topology === 'direct' ? payload : [34, 1, ...payload]));
    for (const [index, payload, reason] of [
      [1, [95, 0, 1, 2, 3], 'configuration length mismatch'],
      [2, [95, 1, 1, 2, 3], 'custom configuration index must be 0'],
    ] as const) {
      fixture.emit({ kind: 'WRITE', bytes: route([...payload]) });
      await eventually(() => coordinator.snapshot().counters.dropped === index);
      const error = coordinator.snapshot().lastAttempt?.error;
      assert.ok(error?.includes(reason), error);
      assert.ok(
        logs.some(
          (line) =>
            line.includes(`command=95 route=${topology === 'direct' ? 'direct' : 'CAN:1'}`) && line.includes(reason)
        )
      );
    }
    await coordinator.act({ kind: 'start-riding' });
    fixture.emit({ kind: 'WRITE', bytes: route([95, 0]) });
    await eventually(() => coordinator.snapshot().counters.dropped === 3);
    assert.match(coordinator.snapshot().lastAttempt?.error ?? '', /controller is not idle/);
    assert.equal(coordinator.snapshot().serial.status, 'attached');
    await coordinator.cleanup();
  }
});

test('ports are bounded and initial attachment failure becomes detached state', async () => {
  const coordinator = new FixtureCoordinator({
    open: async () => {
      throw new Error('Port occupied');
    },
    listPorts: async () =>
      Array.from({ length: 40 }, (_, index) => ({
        path: `/dev/${index}`,
        label: `Fixture ${index}`,
      })),
    log: () => undefined,
  });
  assert.equal((await coordinator.ports()).length, 32);
  await coordinator.initialise('/dev/missing');
  assert.equal(coordinator.snapshot().serial.status, 'detached');
  assert.match(coordinator.snapshot().serial.error ?? '', /Port occupied/);
});

test('process resolver returns one coordinator across repeated route resolution', () => {
  assert.equal(fixtureCoordinator(), fixtureCoordinator());
});

function coordinatorFor(
  fixture: FakeFixture,
  log: (level: 'info' | 'warn' | 'error', message: string) => void = () => undefined
) {
  return new FixtureCoordinator({
    open: async () => fixture,
    listPorts: async () => [{ path: '/dev/fixture', label: 'Fixture' }],
    log,
  });
}

function configuration(overrides: Partial<FixtureConfiguration> = {}): FixtureConfiguration {
  return {
    topology: 'express',
    canId: 1,
    firmware: '6.06',
    refloat: '1.2.3',
    bmsTopology: 'none',
    bmsCanId: 42,
    bmsVariant: 'extended',
    bmsFailureMode: 'none',
    bmsCellCount: 15,
    bmsTemperatureCount: 3,
    bmsStateOfCharge: 0.75,
    bmsInputCurrentA: 0,
    ...overrides,
  };
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
