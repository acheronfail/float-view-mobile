import assert from 'node:assert/strict';
import test from 'node:test';
import {
  MOCK_PORT,
  VITE_PORT,
  androidInstallArguments,
  androidLaunchArguments,
  androidLogcatArguments,
  childExitFailure,
  iosPhysicalConsoleLaunchArguments,
  iosSimulatorBuildArguments,
  iosSimulatorConsoleLaunchArguments,
  selectAndroidDeviceFromList,
  selectIOSSimulatorFromList,
  waitForURL,
  viteEnvironment,
} from '../src/dev.ts';

test('Vite receives the discovered connection address and device auth origin', () => {
  assert.equal(VITE_PORT, 11002);
  assert.equal(MOCK_PORT, 11003);
  const environment = viteEnvironment(
    { url: 'ws://127.0.0.1:49300/api/ws', origin: 'http://127.0.0.1:49200' },
    { KEEP: 'yes' }
  );
  assert.equal(environment.FLOAT_VIEW_DEV_BACKEND_ORIGIN, 'http://127.0.0.1:49300');
  assert.equal(environment.FLOAT_VIEW_DEV_BACKEND_AUTH_ORIGIN, 'http://127.0.0.1:49200');
  assert.equal(environment.KEEP, 'yes');
  assert.equal(viteEnvironment(undefined, {}).FLOAT_VIEW_DEV_BACKEND_ORIGIN, 'http://127.0.0.1:11003');
});

test('Android development preserves app data and requires explicit ambiguous selection', () => {
  assert.deepEqual(androidInstallArguments('/repo'), [
    'install',
    '-r',
    '/repo/backends/android/app/build/outputs/apk/debug/app-debug.apk',
  ]);
  assert.deepEqual(androidLaunchArguments(), [
    'shell',
    'am',
    'start',
    '-W',
    '-n',
    'fail.acheron.floatview/fail.acheron.floatview.ui.MainActivity',
  ]);
  assert.deepEqual(androidLogcatArguments('1234'), ['logcat', '--pid', '1234', '-v', 'color']);
  const multiple = 'List of devices attached\none\tdevice\ntwo\tdevice\n';
  assert.throws(() => selectAndroidDeviceFromList(multiple), /Pass an Android target/);
  assert.equal(selectAndroidDeviceFromList(multiple, 'two'), 'two');
});

test('iOS development prefers a booted simulator and forwards logs', () => {
  const devices = {
    devices: {
      'iOS 26': [
        { udid: 'shutdown', state: 'Shutdown' },
        { udid: 'booted', state: 'Booted' },
      ],
    },
  };
  assert.equal(selectIOSSimulatorFromList(devices), 'booted');
  assert.deepEqual(iosSimulatorBuildArguments('/app.xcodeproj', 'booted', '/derived'), [
    '-project',
    '/app.xcodeproj',
    '-scheme',
    'float-view',
    '-sdk',
    'iphonesimulator',
    '-destination',
    'id=booted',
    '-derivedDataPath',
    '/derived',
    'CODE_SIGNING_ALLOWED=NO',
    'build',
  ]);
  assert.deepEqual(iosSimulatorConsoleLaunchArguments('booted'), [
    'simctl',
    'launch',
    '--terminate-running-process',
    '--console',
    'booted',
    'fail.acheron.floatview',
  ]);
  assert.deepEqual(iosPhysicalConsoleLaunchArguments('physical'), [
    'devicectl',
    'device',
    'process',
    'launch',
    '--device',
    'physical',
    '--terminate-existing',
    '--console',
    'fail.acheron.floatview',
  ]);
});

test('readiness and supervised child failures are bounded and descriptive', async () => {
  await assert.rejects(
    waitForURL('http://127.0.0.1:1/unavailable', () => undefined, 20),
    /did not become ready/
  );
  const childFailure = childExitFailure('iOS iproxy on 127.0.0.1:11011', 2, null);
  assert.match(childFailure?.message ?? '', /iproxy.*code 2/);
  assert.equal(childExitFailure('owned child', null, 'SIGTERM'), undefined);
});
