// Focused OS permission checkpoint. The existing ESP32 identity checkpoint is separate.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { androidPhysicalTarget } from '../targets/android.ts';
import { ConformanceClient, ConformanceError } from './client.ts';
import { validatePermissions } from '../../frontend/src/lib/backend/permissions.ts';

const serial = process.argv[2] ?? '';
if (!serial) throw new Error('Pass the connected Android serial');
const target = androidPhysicalTarget(serial);
const app = 'fail.acheron.floatview';
const background = 'android.permission.ACCESS_BACKGROUND_LOCATION';
function adb(...args: string[]) {
  return execFileSync('adb', ['-s', serial, ...args], { encoding: 'utf8', timeout: 15_000 });
}
function shell(...args: string[]) {
  return adb('shell', ...args);
}
const initialGrants = shell('dumpsys', 'package', app);
assert.match(
  initialGrants,
  /ACCESS_BACKGROUND_LOCATION: granted=true/,
  'Checkpoint currently requires initial background grant for deterministic restore'
);
assert.equal(shell('getprop', 'ro.build.version.sdk').trim(), '29', 'This prompt checkpoint targets API 29');
let client: ConformanceClient | undefined;
async function connect() {
  client?.close();
  await target.foreground!();
  const e = await target.endpoint();
  client = await ConformanceClient.open(e.url, e.origin);
  await tap(/content-desc="settings"|text="settings"/i);
  await client.waitForState((s) => !s.bluetooth.scanning, { timeoutMs: 15_000 });
  return client;
}
async function tap(text: RegExp) {
  const end = Date.now() + 12_000;
  while (Date.now() < end) {
    shell('uiautomator', 'dump', '/sdcard/float-view-permissions.xml');
    const xml = shell('cat', '/sdcard/float-view-permissions.xml');
    for (const node of xml.match(/<node\b[^>]+>/g) ?? []) {
      if (!text.test(node)) continue;
      const bounds = /bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/.exec(node);
      if (bounds) {
        shell(
          'input',
          'tap',
          String((Number(bounds[1]) + Number(bounds[3])) / 2),
          String((Number(bounds[2]) + Number(bounds[4])) / 2)
        );
        return;
      }
    }
    await new Promise((r) => setTimeout(r, 250));
  }
  throw new Error(`OS control did not appear: ${text}`);
}
try {
  await target.prepareSuite!(console.log);
  await target.start();
  await connect();
  const initial = await client!.request('state.get', {});
  validatePermissions(initial.permissions);
  assert.deepEqual(
    initial.permissions.map((r) => r.permission),
    ['location', 'backgroundLocation']
  );
  assert.equal(initial.settings.locationRecordingEnabled, false);
  shell('pm', 'revoke', app, background);
  await connect();
  let state = await client!.request('state.get', {});
  if (state.permissions[1]!.action === 'requestPermission') {
    assert.equal(state.permissions[1]!.action, 'requestPermission');
    const request = client!.request(
      'permissions.recover',
      { permission: 'backgroundLocation', action: 'requestPermission' },
      35_000
    );
    console.log('initial recovery', state.permissions[1]);
    if (state.permissions[1]!.guidance === 'rationale') await tap(/text="Continue"/i);
    await tap(/permission_deny_button/);
    await request;
    state = await client!.request('state.get', {});
    console.log('observed background', state.permissions[1]);
    assert.equal(state.permissions[1]!.status, 'denied');
    assert.equal(state.settings.locationRecordingEnabled, false);
    assert.equal(state.bluetooth.scanning, false);
    console.log('PASS: separate API 29 background dialog, denial, unchanged recording and scanning');
    if (state.permissions[1]!.action === 'requestPermission') {
      const retry = client!.request(
        'permissions.recover',
        { permission: 'backgroundLocation', action: 'requestPermission' },
        35_000
      );
      await tap(/text="Continue"/i);
      await tap(/permission_allow_always_button|text="Allow all the time"/i);
      await retry;
      state = await client!.request('state.get', {});
      assert.equal(state.permissions[1]!.status, 'granted');
      console.log('PASS: rationale and explicit retry to Always');
    }
  } else console.log('Native retry is already blocked by retained OS history; checking Settings recovery.');
  shell('pm', 'revoke', app, background);

  await connect();
  state = await client!.request('state.get', {});
  assert.equal(state.permissions[1]!.action, 'openSettings');
  await assert.rejects(
    client!.request('permissions.recover', { permission: 'backgroundLocation', action: 'requestPermission' }),
    (e: unknown) => e instanceof ConformanceError && e.error.code === 'invalid_state'
  );
  const navigate = client!.request(
    'permissions.recover',
    { permission: 'backgroundLocation', action: 'openSettings' },
    35_000
  );
  await tap(/text="Open settings"/i);
  await navigate;
  assert.match(shell('dumpsys', 'activity', 'activities'), /mResumedActivity.*com.android.settings/);
  shell('input', 'keyevent', 'KEYCODE_BACK');
  await connect();
  state = await client!.request('state.get', {});
  assert.equal(state.permissions[1]!.status, 'denied');
  console.log('PASS: blocked retry routes to app Settings; return without grant remains denied');

  shell('pm', 'grant', app, background);
  await target.background!();
  await connect();
  state = await client!.request('state.get', {});
  assert.equal(state.permissions[1]!.status, 'granted');
  assert.equal(state.settings.locationRecordingEnabled, false);
  console.log('PASS: activation observes external grant with recording disabled');
} catch (error) {
  console.error('CHECKPOINT FAILURE', error);
  throw error;
} finally {
  client?.close();

  shell('pm', 'grant', app, background);
  shell('rm', '-f', '/sdcard/float-view-permissions.xml');
  assert.match(shell('dumpsys', 'package', app), /ACCESS_BACKGROUND_LOCATION: granted=true/);
  await target.finishSuite!(console.log);
  console.log('RESTORED: original background grant; UI dump removed. OS denial history is retained.');
}
