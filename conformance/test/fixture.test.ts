import assert from 'node:assert/strict';
import test from 'node:test';
import { expressThor } from '../fixture/boards.ts';
import { crc16, encodeFrame, FrameAssembler } from '../fixture/frame.ts';
import { FixtureRunner, chunks } from '../fixture/runner.ts';
import { fixtureScenarios, physicalScenarioMatrix } from '../fixture/scenarios.ts';
import { led, parseEvent, type FixtureControl, type FixtureEvent } from '../fixture/serial.ts';
import { VescSimulator } from '../fixture/simulator.ts';

test('frame assembler handles fragmentation and validates CRC', () => {
  const frame = encodeFrame(Uint8Array.of(0));
  assert.equal(crc16(Uint8Array.of(0)), 0);
  const assembler = new FrameAssembler();
  assert.deepEqual(assembler.append(frame.slice(0, 2)), []);
  assert.deepEqual(assembler.append(frame.slice(2)), [Uint8Array.of(0)]);
});

test('simulator follows exact Express, CAN, Refloat, and telemetry sequence', () => {
  let now = 0;
  const simulator = new VescSimulator(expressThor, () => now);
  assert.equal(
    hex(response(simulator, [0])),
    '0227000605564553432045787072657373205400000000000000000000000000000002000000000000bf7003'
  );
  assert.equal(response(simulator, [62]).request.kind, 'can-ping');
  const controller = response(simulator, [34, 1, 0]);
  assert.equal(controller.request.kind, 'controller-firmware');
  assert.equal(hex(controller), '022400060654484f5233303000000000000000000000000000000000000000000000000000006adf03');
  assert.equal(response(simulator, [34, 1, 14]).request.kind, 'motor-configuration');
  assert.equal(response(simulator, [34, 1, 36, 101, 0, 2, 0]).request.kind, 'refloat-info');
  assert.equal(response(simulator, [34, 1, 36, 101, 32]).request.kind, 'refloat-layout');
  assert.deepEqual(
    [...new FrameAssembler().append(response(simulator, [34, 1, 36, 101, 20, 0, 0, 0, 0, 0]).frame)[0]!],
    [36, 101, 20, 3]
  );
  for (const time of [100, 200, 300, 400]) {
    now = time;
    assert.equal(response(simulator, [34, 1, 36, 101, 31]).request.telemetryPhase, 'idle');
  }
  for (const time of [500, 600, 700]) {
    now = time;
    assert.equal(response(simulator, [34, 1, 36, 101, 31]).request.telemetryPhase, 'riding');
  }
  simulator.verifyCadence(100, 0);
  assert.throws(() => response(simulator, [6]), /Unsafe or unsupported/);
  assert.throws(() => response(simulator, [34, 9, 0]), /Unknown CAN target/);
});

test('serial events and diagnostic LED conventions are bounded', () => {
  assert.deepEqual(parseEvent('READY'), { kind: 'READY' });
  assert.deepEqual(parseEvent('WRITE 00AFF0'), {
    kind: 'WRITE',
    bytes: Uint8Array.of(0, 0xaf, 0xf0),
  });
  assert.equal(led.off, '000000');
  assert.throws(() => parseEvent('WRITE xyz'));
  assert.throws(() => chunks(Uint8Array.of(1), 245), /fragment bound/);
  assert.deepEqual(
    chunks(
      Uint8Array.from({ length: 45 }, (_, index) => index),
      20
    ).map((item) => item.length),
    [20, 20, 5]
  );
});

test('fixture runner concurrently answers writes and re-advertises after injection', async () => {
  const fixture = new FakeFixture();
  const progress: string[] = [];
  const runner = new FixtureRunner(fixtureScenarios['unexpected-disconnect'], fixture, undefined, 1_000, (message) =>
    progress.push(message)
  );
  await runner.start();
  fixture.emit({ kind: 'CONNECTED' });
  for (const payload of [
    [0],
    [62],
    [34, 1, 0],
    [34, 1, 36, 101, 0, 2, 0],
    [34, 1, 36, 101, 32],
    [34, 1, 36, 101, 31],
    [34, 1, 36, 101, 31],
    [34, 1, 36, 101, 31],
    [34, 1, 36, 101, 31],
  ]) {
    fixture.emit({
      kind: 'WRITE',
      bytes: encodeFrame(Uint8Array.from(payload)),
    });
  }
  await runner.waitForRequests((requests) => requests.filter((item) => item.kind === 'telemetry').length === 4);
  assert.equal(fixture.commands.filter((item) => item === 'DISCONNECT').length, 1);
  await eventually(() => fixture.commands.filter((item) => item === 'ADVERTISE').length === 2);
  const responsesBeforeReconnect = fixture.notifications.length;
  fixture.emit({ kind: 'WRITE', bytes: encodeFrame(Uint8Array.of(0)) });
  await eventually(() => fixture.notifications.length > responsesBeforeReconnect);
  assert.equal(fixture.commands.filter((item) => item === 'DISCONNECT').length, 1);
  assert.ok(fixture.notifications.length > 4);
  assert.equal(runner.writePayloadMaximum, 12);
  assert.ok(progress.includes('ESP < WRITE payload_max=12'));
  await runner.stop(true);
  assert.equal(fixture.commands.filter((item) => item === 'RESET').length, 2);
  assert.equal(fixture.colors.at(-1), led.off);
  for (const expected of [
    'ESP > RESET',
    'ESP < READY',
    'ESP > ADVERTISE',
    'ESP < CONNECTED',
    'ESP < WRITE transport-firmware',
    'ESP > NOTIFY transport-firmware',
    'ESP > DISCONNECT',
    'ESP < DISCONNECTED',
    'ESP > LED 000000 (off)',
  ]) {
    assert.ok(
      progress.some((message) => message.startsWith(expected)),
      `Missing fixture progress: ${expected}`
    );
  }
});

test('fixture runner tolerates a late notification after the peer disconnects', async () => {
  const fixture = new FakeFixture();
  const runner = new FixtureRunner(fixtureScenarios.identity, fixture);
  await runner.start();
  fixture.emit({ kind: 'CONNECTED' });
  fixture.emit({ kind: 'DISCONNECTED' });
  fixture.emit({ kind: 'ERROR', code: 'NOT_CONNECTED' });
  await eventually(() => runner.diagnosticSnapshot().includes('ERROR'));
  runner.verify();
  await runner.stop(true);
});

test('fixture runner turns an accepted shutdown intent into controller loss without re-advertising', async () => {
  const fixture = new FakeFixture();
  const runner = new FixtureRunner(fixtureScenarios['shutdown-direct'], fixture);
  await runner.start();
  fixture.emit({ kind: 'CONNECTED' });
  fixture.emit({ kind: 'WRITE', bytes: encodeFrame(Uint8Array.of(156, 0, 0)) });
  await eventually(() => fixture.commands.includes('DISCONNECT'));
  assert.equal(runner.simulator.requests.filter((request) => request.kind === 'controller-shutdown').length, 1);
  assert.equal(fixture.commands.filter((command) => command === 'ADVERTISE').length, 1);
  await runner.stop(true);
});

test('fixture runner retries reset when serial-open reboot loses initial commands', async () => {
  const fixture = new FakeFixture(3);
  const runner = new FixtureRunner(fixtureScenarios.identity, fixture, undefined, 5);
  await runner.start();
  assert.equal(fixture.commands.filter((command) => command === 'RESET').length, 3);
  await runner.stop(true);
});

test('fixture startup advertising retry is bounded and preserves persistent failure', async () => {
  const fixture = new FakeFixture();
  const runner = new FixtureRunner(fixtureScenarios.identity, fixture, undefined, 5);
  await runner.start();
  for (let attempt = 1; attempt <= 3; attempt += 1) {
    fixture.emit({ kind: 'ERROR', code: 'ADVERTISE_FAILED' });
    await eventually(() => fixture.commands.filter((command) => command === 'ADVERTISE').length === attempt + 1);
  }
  fixture.emit({ kind: 'ERROR', code: 'ADVERTISE_FAILED' });
  await eventually(() => fixture.colors.includes(led.failed));
  assert.equal(fixture.commands.filter((command) => command === 'ADVERTISE').length, 4);
  assert.throws(() => runner.verify(), /ADVERTISE_FAILED/);
  await runner.stop(false);
});

test('physical scenario matrix excludes desktop backgrounding', () => {
  const connectionScenarios = [
    'identity',
    'guest',
    'direct',
    'direct-legacy',
    'fault-direct',
    'fault-can',
    'imu-timeout-beta2',
    'control-direct',
    'control-can',
    'shutdown-direct',
    'shutdown-can',
    'shutdown-no-op',
    'explicit-can',
    'generic-gateway',
    'ambiguous',
    'bms-direct-controller',
    'bms-express',
    'bms-standalone',
    'ride',
    'unexpected-disconnect',
  ];
  assert.deepEqual(physicalScenarioMatrix.macos, connectionScenarios);
  assert.deepEqual(physicalScenarioMatrix.ios, [...connectionScenarios, 'background', 'auto-connect']);
  assert.deepEqual(physicalScenarioMatrix.android, [...connectionScenarios, 'background', 'auto-connect']);
  assert.ok(physicalScenarioMatrix.android.includes('background'));
  assert.ok(physicalScenarioMatrix.ios.includes('background'));
});

function response(simulator: VescSimulator, payload: number[]) {
  const value = simulator.receive(encodeFrame(Uint8Array.from(payload)));
  assert.equal(value.length, 1);
  return value[0]!;
}

function hex(value: { frame: Uint8Array }) {
  return Buffer.from(value.frame).toString('hex');
}

async function eventually(predicate: () => boolean) {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 1));
  }
  throw new Error('Condition did not become true');
}

class FakeFixture implements FixtureControl {
  private readonly readyOnReset: number;
  readonly commands: string[] = [];
  readonly notifications: Uint8Array[] = [];
  readonly colors: string[] = [];
  private readonly events: FixtureEvent[] = [];
  private waiter?: (event: FixtureEvent) => void;
  private waiterReject?: (error: Error) => void;
  private waiterTimer?: NodeJS.Timeout;

  constructor(readyOnReset = 1) {
    this.readyOnReset = readyOnReset;
  }

  async command(command: 'RESET' | 'ADVERTISE' | 'DISCONNECT') {
    this.commands.push(command);
    if (command === 'RESET' && this.commands.filter((candidate) => candidate === 'RESET').length >= this.readyOnReset) {
      this.emit({ kind: 'READY' });
    }
    if (command === 'DISCONNECT') this.emit({ kind: 'DISCONNECTED' });
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
      this.waiter = resolve;
      this.waiterReject = reject;
      this.waiterTimer = setTimeout(() => {
        if (this.waiter !== resolve) return;
        this.waiter = undefined;
        this.waiterReject = undefined;
        this.waiterTimer = undefined;
        reject(new Error('Timed out waiting for fake fixture'));
      }, timeoutMs);
    });
  }

  emit(event: FixtureEvent) {
    if (this.waiter) {
      const resolve = this.waiter;
      if (this.waiterTimer) clearTimeout(this.waiterTimer);
      this.waiter = undefined;
      this.waiterReject = undefined;
      this.waiterTimer = undefined;
      resolve(event);
    } else {
      this.events.push(event);
    }
  }

  async close() {
    if (this.waiterTimer) clearTimeout(this.waiterTimer);
    this.waiterReject?.(new Error('Fixture serial client closed'));
    this.waiter = undefined;
    this.waiterReject = undefined;
    this.waiterTimer = undefined;
  }
}
