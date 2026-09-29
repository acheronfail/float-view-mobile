import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import { SoftwarePeripheral } from '../fixture/software.ts';
import { encodeFrame, FrameAssembler } from '../fixture/frame.ts';
import { FixtureRun, decodeBytes, checkEventCapacity, LIMITS, type FixtureCommand } from '../fixture/session.ts';
import { startFixtureRelay, validateRelayEndpoint } from '../fixture/relay.ts';

const fixture = (path: string) => readFileSync(new URL(`../../backends/fixtures/${path}`, import.meta.url));
const request = (...bytes: number[]) => encodeFrame(Uint8Array.from(bytes));
const payload = (frame: Uint8Array) => new FrameAssembler().append(frame)[0]!;
test('software golden transcript matches independent source vectors and synthetic metadata', () => {
  const model = new SoftwarePeripheral();
  const respond = (...bytes: number[]) => payload(model.receive(request(...bytes))[0]!);
  assert.equal(
    Buffer.from(respond(0)).toString('hex'),
    '00060654484f523330300000000000000000000000000000000001000000005645534300'
  );
  assert.deepEqual(respond(62), Uint8Array.of(62, 42));
  const bms = JSON.parse(fixture('vesc-bms/source-derived.json').toString());
  for (const [command, name] of [
    [0, 'vbms-firmware-6.00'],
    [96, 'vbms-values-extended-typical'],
  ] as const)
    assert.equal(
      Buffer.from(respond(34, 42, command)).toString('hex'),
      bms.cases.find((c: { name: string }) => c.name === name).payloadHex
    );
  const info = JSON.parse(fixture('refloat/protocol-1.0.0.json').toString()).cases[0];
  assert.equal(Buffer.from(model.receive(request(...info.request))[0]!).toString('hex'), info.frameHex);
  assert.equal(
    Buffer.from(respond(36, 101, 10, 4)).toString('hex'),
    '24650a0300000000000001000000000000000000000000020400000000000000008000000000003c380000000000000000000000000000'
  );
  assert.deepEqual(model.receive(request(36, 101, 0, 99)), []);
});
test('configuration write/readback/reset and fragmented, multiple, noise, malformed frames', () => {
  const model = new SoftwarePeripheral();
  const get = () => Buffer.from(payload(model.receive(request(93, 0))[0]!)).subarray(2);
  const initial = fixture('refloat-configuration/values/1.0.0-current.bin');
  assert.deepEqual(get(), initial);
  const write = request(95, 0, ...fixture('refloat-configuration/values/1.0.0-write.bin'));
  assert.deepEqual(model.receive(write.subarray(0, 9)), []);
  assert.deepEqual(payload(model.receive(write.subarray(9))[0]!), Uint8Array.of(95));
  assert.deepEqual(get(), fixture('refloat-configuration/values/1.0.0-readback.bin'));
  model.disconnect();
  assert.notDeepEqual(get(), initial);
  model.reset();
  assert.deepEqual(get(), initial);
  assert.equal(model.receive(Uint8Array.from([99, ...request(0), ...request(62)])).length, 2);
  const bad = request(0);
  bad[3] = 255;
  assert.throws(() => model.receive(bad), /CRC/);
  model.reset();
  assert.throws(() => model.receive(Uint8Array.of(3, 255, 255)), /bound/);
});
function client(run = new FixtureRun()) {
  const session = run.create();
  let operation = 0,
    generation = 0;
  return {
    run,
    session,
    send(command: FixtureCommand['command'], extra = {}) {
      if (command === 'connect') generation++;
      return run.execute({
        run: run.id,
        session: session.id,
        operation: ++operation,
        generation,
        command,
        transport_operation: operation,
        response: true,
        device: 'software-vesc',
        ...extra,
      });
    },
  };
}
test('session ordering, foreign/duplicate rejection, reconnect and isolated reset', () => {
  const a = client();
  const b = client(a.run);
  const scan = a.send('scan');
  assert.equal(scan.events[0]?.name, 'Software VESC');
  a.send('connect');
  const bytes = Buffer.from(request(62)).toString('base64');
  const reply = a.send('write', { bytes });
  assert.throws(() => a.run.execute({ ...reply, command: 'write', device: 'software-vesc', bytes }), /duplicate/);
  assert.throws(() => a.run.execute({ ...reply, run: 'foreign', operation: 4, command: 'reset' }), /Foreign/);
  a.send('disconnect');
  a.send('connect');
  const next = a.send('write', { bytes });
  assert.equal(next.generation, 2);
  assert.ok(next.events[0]!.sequence > reply.events[0]!.sequence);
  b.send('connect');
  assert.equal(b.send('write', { bytes }).generation, 1);
  a.send('close');
  b.send('poll');
  const fresh = client();
  assert.notEqual(fresh.run.token, a.run.token);
  a.run.close();
  fresh.run.close();
});
test('exact byte/session/event limits and cleanup after saturation', () => {
  assert.equal(decodeBytes(Buffer.alloc(LIMITS.payload).toString('base64')).length, LIMITS.payload);
  assert.throws(() => decodeBytes(Buffer.alloc(LIMITS.payload + 1).toString('base64')), /bounded/);
  assert.throws(() => decodeBytes('!!!!'), /bounded/);
  const run = new FixtureRun();
  for (let n = 0; n < 4; n++) run.create();
  assert.throws(() => run.create(), /capacity/);
  run.close();
  run.create();
  run.close();
  for (const count of [126, 127]) {
    const c = client();
    c.send('connect');
    const bytes = Buffer.concat(Array.from({ length: count }, () => request(62))).toString('base64');
    if (count === 126) assert.equal(c.send('write', { bytes }).events.length, count + 2);
    else assert.throws(() => c.send('write', { bytes }), /capacity/);
    c.session.close();
    c.run.close();
  }
});
test('HTTP authorization, endpoint selection, body limits and bounded shutdown', async () => {
  for (const endpoint of [
    'http://localhost:1234/',
    'http://192.168.1.1:1234/',
    'https://127.0.0.1:1234/',
    'http://user@127.0.0.1:1234/',
  ])
    assert.throws(() => validateRelayEndpoint(endpoint));
  const relay = await startFixtureRelay();
  try {
    validateRelayEndpoint(relay.endpoint);
    const input = { run: relay.run, session: relay.session, operation: 1, generation: 0, command: 'scan' };
    const send = (token: string, body: string) =>
      fetch(relay.endpoint, {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${token}`,
          'X-Fixture-Session': relay.session,
        },
        body,
      });
    assert.equal((await send('wrong', JSON.stringify(input))).status, 403);
    const good = await send(relay.token, JSON.stringify(input));
    assert.equal(good.status, 200);
    assert.equal((await good.json()).events[0].event, 'discovered');
    const exact = JSON.stringify({ ...input, operation: 2, command: 'poll' });
    assert.equal((await send(relay.token, exact + ' '.repeat(LIMITS.message - exact.length))).status, 200);
    assert.equal((await send(relay.token, ' '.repeat(LIMITS.message + 1))).status, 400);
  } finally {
    const start = performance.now();
    await relay.close();
    assert.ok(performance.now() - start < 5_000);
  }
});

test('HTTP sessions stay isolated and duplicate writes do not advance protocol state', async () => {
  const relay = await startFixtureRelay();
  try {
    const other = relay.createSession();
    const send = (session: string, operation: number, command: string, generation = 0, extra = {}) =>
      fetch(relay.endpoint, {
        method: 'POST',
        headers: { Authorization: `Bearer ${relay.token}`, 'X-Fixture-Session': session },
        body: JSON.stringify({
          run: relay.run,
          session,
          operation,
          generation,
          command,
          transport_operation: operation,
          response: true,
          ...extra,
        }),
      });
    assert.equal((await send(relay.session, 1, 'connect', 1, { device: 'software-vesc' })).status, 200);
    const bytes = Buffer.from(request(95, 0, ...fixture('refloat-configuration/values/1.0.0-write.bin'))).toString(
      'base64'
    );
    assert.equal((await send(relay.session, 1, 'write', 1, { device: 'software-vesc', bytes })).status, 400);
    const result = await send(relay.session, 2, 'write', 1, {
      device: 'software-vesc',
      bytes: Buffer.from(request(93, 0)).toString('base64'),
    });
    assert.equal(result.status, 200);
    const reply = await result.json();
    assert.deepEqual(
      Buffer.from(
        payload(Buffer.from(reply.events.find((e: { event: string }) => e.event === 'bytes').bytes, 'base64'))
      ).subarray(2),
      fixture('refloat-configuration/values/1.0.0-current.bin')
    );
    assert.equal((await send(other, 1, 'scan')).status, 200);
  } finally {
    await relay.close();
  }
});

test('64 pending commands admit the exact limit, reject +1, and reserve close and shutdown', async () => {
  const { createConnection } = await import('node:net');
  const relay = await startFixtureRelay();
  const sockets: import('node:net').Socket[] = [];
  const port = Number(new URL(relay.endpoint).port);
  const stalled = async () => {
    const socket = createConnection({ host: '127.0.0.1', port });
    socket.on('error', () => {});
    sockets.push(socket);
    await new Promise<void>((resolve) => socket.once('connect', resolve));
    socket.write(
      `POST / HTTP/1.1\r\nHost: 127.0.0.1:${port}\r\nAuthorization: Bearer ${relay.token}\r\nX-Fixture-Session: ${relay.session}\r\nContent-Length: 100\r\n\r\n{`
    );
    await new Promise((resolve) => setTimeout(resolve, 2));
  };
  const send = (operation: number, command: string, cleanup = false) =>
    fetch(relay.endpoint, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${relay.token}`,
        'X-Fixture-Session': relay.session,
        ...(cleanup ? { 'X-Fixture-Close': '1' } : {}),
      },
      body: JSON.stringify({ run: relay.run, session: relay.session, operation, generation: 0, command }),
    });
  try {
    for (let i = 0; i < 63; i++) await stalled();
    assert.equal((await send(1, 'poll')).status, 200);
    await stalled();
    assert.equal((await send(2, 'poll')).status, 429);
    assert.equal((await send(2, 'close', true)).status, 200);
  } finally {
    const began = performance.now();
    await relay.close();
    for (const socket of sockets) socket.destroy();
    assert.ok(performance.now() - began < 5_000);
  }
});

test('aggregate event bytes, encoded reply boundary, and old-generation queued events', () => {
  checkEventCapacity(128, 65_536);
  assert.throws(() => checkEventCapacity(129, 65_536), /capacity/);
  assert.throws(() => checkEventCapacity(128, 65_537), /capacity/);
  const c = client();
  c.send('connect');
  const schema = request(92, 0, 0, 0, 1, 144, 0, 0, 0, 0);
  const bytes = Buffer.concat(Array.from({ length: 126 }, () => schema)).toString('base64');
  const first = c.send('write', { bytes });
  assert.ok(Buffer.byteLength(JSON.stringify(first)) <= LIMITS.message);
  assert.ok(first.events.length < 128);
  const rest = c.send('poll');
  assert.equal(first.events.length + rest.events.length, 128);
  assert.ok(rest.events[0]!.sequence > first.events.at(-1)!.sequence);
  c.send('write', { bytes });
  c.send('disconnect');
  c.send('connect');
  assert.deepEqual(c.send('poll').events, []);
  assert.throws(
    () =>
      c.run.execute({
        run: c.run.id,
        session: c.session.id,
        operation: 8,
        generation: 1,
        command: 'write',
        device: 'software-vesc',
        bytes,
      }),
    /Stale/
  );
  c.send('reset');
  assert.deepEqual(c.send('poll').events, []);
  c.run.close();
});

test('stalled input expires before five seconds and cannot mutate another session', async () => {
  const { createConnection } = await import('node:net');
  const relay = await startFixtureRelay();
  const port = Number(new URL(relay.endpoint).port);
  const socket = createConnection({ host: '127.0.0.1', port });
  socket.on('error', () => {});
  socket.resume();
  try {
    await new Promise<void>((resolve) => socket.once('connect', resolve));
    const began = performance.now();
    const closed = new Promise<void>((resolve) => socket.once('close', () => resolve()));
    socket.write(
      `POST / HTTP/1.1\r\nHost: 127.0.0.1:${port}\r\nAuthorization: Bearer ${relay.token}\r\nX-Fixture-Session: ${relay.session}\r\nContent-Length: 100\r\n\r\n{`
    );
    await closed;
    assert.ok(performance.now() - began < 5_000);
  } finally {
    socket.destroy();
    await relay.close();
  }
});

test('aggregate pending input is bounded and reserved close works with its byte budget exhausted', async () => {
  const { createConnection } = await import('node:net');
  const relay = await startFixtureRelay();
  const port = Number(new URL(relay.endpoint).port);
  const sockets: import('node:net').Socket[] = [];
  try {
    for (let i = 0; i < 2; i++) {
      const socket = createConnection({ host: '127.0.0.1', port });
      sockets.push(socket);
      socket.on('error', () => {});
      socket.resume();
      await new Promise<void>((resolve) => socket.once('connect', resolve));
      socket.write(
        `POST / HTTP/1.1\r\nHost: 127.0.0.1:${port}\r\nAuthorization: Bearer ${relay.token}\r\nX-Fixture-Session: ${relay.session}\r\nContent-Length: 65536\r\n\r\n${' '.repeat(32768)}`
      );
    }
    await new Promise((resolve) => setTimeout(resolve, 20));
    const closed = await fetch(relay.endpoint, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${relay.token}`,
        'X-Fixture-Session': relay.session,
        'X-Fixture-Close': '1',
      },
      body: JSON.stringify({ run: relay.run, session: relay.session, operation: 1, generation: 0, command: 'close' }),
    });
    assert.equal(closed.status, 200);
  } finally {
    for (const socket of sockets) socket.destroy();
    await relay.close();
  }
});
