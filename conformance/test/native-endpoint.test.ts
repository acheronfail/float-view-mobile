import assert from 'node:assert/strict';
import { PassThrough } from 'node:stream';
import { createServer } from 'node:http';
import { mkdtempSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { androidEndpoint, assignedPort, forwardedBootstrap, iosEndpoint } from '../src/native-endpoint.ts';
import { macosReadiness } from '../src/dev.ts';

test('readiness validates assigned ports and consumes macOS readiness', async () => {
  for (const invalid of ['', '0', '-1', '65536', '1.5', '123 garbage']) assert.throws(() => assignedPort(invalid));
  assert.equal(assignedPort('49321\n'), 49321);
  const stream = new PassThrough();
  const ready = macosReadiness(stream);
  stream.write('{"origin":"http://127.0.0.1:49321",');
  stream.write('"webSocketUrl":"ws://127.0.0.1:49321/api/ws"}\n');
  assert.deepEqual(await ready, { origin: 'http://127.0.0.1:49321', url: 'ws://127.0.0.1:49321/api/ws' });
  const ended = new PassThrough();
  const failure = macosReadiness(ended);
  ended.end();
  await assert.rejects(failure, /exited before readiness/);
});

test('physical iOS discovery owns a loopback proxy and terminates it on cleanup', async () => {
  const scratch = mkdtempSync(join(tmpdir(), 'float-view-iproxy-test-'));
  const previousPath = process.env.PATH;
  const previousScratch = process.env.FLOAT_VIEW_ENDPOINT_TEST;
  const connection = iosEndpoint(() => 'disposable', false);
  try {
    writeFileSync(
      join(scratch, 'xcrun'),
      `#!${process.execPath}
const fs = require('node:fs');
const args = process.argv.slice(2);
fs.writeFileSync(args[args.indexOf('--destination') + 1], '49123');
`,
      { mode: 0o755 }
    );
    writeFileSync(
      join(scratch, 'iproxy'),
      `#!${process.execPath}
const fs = require('node:fs');
const args = process.argv.slice(2);
fs.writeFileSync(process.env.FLOAT_VIEW_ENDPOINT_TEST + '/proxy', JSON.stringify({ pid: process.pid, args }));
const [host, device] = args.at(-1).split(':').map(Number);
require('node:http').createServer((request, response) => {
  response.setHeader('Content-Type', 'application/json');
  response.end(JSON.stringify({ webSocketUrl: 'ws://127.0.0.1:' + device + '/api/ws', sessionToken: 'test-token' }));
}).listen(host, '127.0.0.1');
`,
      { mode: 0o755 }
    );
    process.env.PATH = `${scratch}:${previousPath}`;
    process.env.FLOAT_VIEW_ENDPOINT_TEST = scratch;
    const endpoint = await connection.endpoint();
    assert.equal(endpoint.origin, 'http://127.0.0.1:49123');
    const proxy = JSON.parse(readFileSync(join(scratch, 'proxy'), 'utf8')) as { pid: number; args: string[] };
    assert.deepEqual(proxy.args.slice(0, 4), ['--udid', 'disposable', '--source', '127.0.0.1']);
    assert.equal(proxy.args.at(-1), `${new URL(endpoint.url).port}:49123`);
    await connection.close();
    assert.throws(() => process.kill(proxy.pid, 0), { code: 'ESRCH' });
  } finally {
    await connection.close();
    process.env.PATH = previousPath;
    if (previousScratch === undefined) delete process.env.FLOAT_VIEW_ENDPOINT_TEST;
    else process.env.FLOAT_VIEW_ENDPOINT_TEST = previousScratch;
    rmSync(scratch, { recursive: true, force: true });
  }
});

test('forwarding preserves device origin and replaces only owned Android rules', async (t) => {
  let device = 49123;
  const server = createServer((_request, response) => {
    response.setHeader('Content-Type', 'application/json');
    response.end(JSON.stringify({ webSocketUrl: `ws://127.0.0.1:${device}/api/ws`, sessionToken: 'test-token' }));
  });
  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
  t.after(() => new Promise<void>((resolve) => server.close(() => resolve())));
  const address = server.address();
  assert.ok(address && typeof address !== 'string');
  assert.deepEqual(await forwardedBootstrap(address.port, device), {
    url: `ws://127.0.0.1:${address.port}/api/ws?token=test-token`,
    origin: 'http://127.0.0.1:49123',
  });
  await assert.rejects(forwardedBootstrap(address.port, device + 1), /does not match/);
  const scratch = mkdtempSync(join(tmpdir(), 'float-view-adb-test-'));
  const previousPath = process.env.PATH;
  const previousScratch = process.env.FLOAT_VIEW_ENDPOINT_TEST;
  try {
    writeFileSync(join(scratch, 'device'), String(device));
    writeFileSync(join(scratch, 'host'), String(address.port));
    writeFileSync(
      join(scratch, 'adb'),
      `#!/bin/sh
printf '%s\\n' "$*" >> "$FLOAT_VIEW_ENDPOINT_TEST/commands"
case "$*" in
  *'cat files/backend-port') cat "$FLOAT_VIEW_ENDPOINT_TEST/device" ;;
  *'forward tcp:0 '*) cat "$FLOAT_VIEW_ENDPOINT_TEST/host" ;;
esac
`,
      { mode: 0o755 }
    );
    process.env.PATH = `${scratch}:${previousPath}`;
    process.env.FLOAT_VIEW_ENDPOINT_TEST = scratch;
    const connection = androidEndpoint(() => 'disposable');
    await connection.endpoint();
    await connection.endpoint();
    device++;
    writeFileSync(join(scratch, 'device'), String(device));
    assert.equal((await connection.endpoint()).origin, `http://127.0.0.1:${device}`);
    connection.close();
    const commands = readFileSync(join(scratch, 'commands'), 'utf8').trim().split('\n');
    assert.equal(commands.filter((line) => line.includes('forward tcp:0')).length, 2);
    assert.deepEqual(
      commands.filter((line) => line.includes('forward --remove')),
      [`-s disposable forward --remove tcp:${address.port}`, `-s disposable forward --remove tcp:${address.port}`]
    );
    assert.ok(commands.every((line) => !line.includes('--remove-all')));
  } finally {
    process.env.PATH = previousPath;
    if (previousScratch === undefined) delete process.env.FLOAT_VIEW_ENDPOINT_TEST;
    else process.env.FLOAT_VIEW_ENDPOINT_TEST = previousScratch;
    rmSync(scratch, { recursive: true, force: true });
  }
});
