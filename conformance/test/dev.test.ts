import assert from 'node:assert/strict';
import test from 'node:test';
import {
  ANDROID_PORT,
  IOS_PORT,
  MACOS_PORT,
  MOCK_PORT,
  VITE_PORT,
  androidForwardArguments,
  androidForwardRemovalArguments,
  androidInstallArguments,
  androidLaunchArguments,
  backendOrigin,
  childExitFailure,
  iosPhysicalConsoleLaunchArguments,
  iosProxyArguments,
  iosSimulatorBuildArguments,
  iosSimulatorConsoleLaunchArguments,
  selectAndroidDeviceFromList,
  selectIOSSimulatorFromList,
  waitForURL,
  viteEnvironment,
} from '../src/dev.ts';

test('development endpoints use the stable registry', () => {
  assert.equal(VITE_PORT, 11002);
  assert.equal(MOCK_PORT, 11003);
  assert.equal(MACOS_PORT, 11010);
  assert.equal(IOS_PORT, 11011);
  assert.equal(ANDROID_PORT, 11012);
  assert.equal(backendOrigin('macos'), 'http://127.0.0.1:11010');
  assert.equal(backendOrigin('ios'), 'http://127.0.0.1:11011');
  assert.equal(backendOrigin('android'), 'http://127.0.0.1:11012');
});

test('mobile forwarding preserves platform ports and owned cleanup scope', () => {
  assert.deepEqual(androidForwardArguments(), ['forward', '--no-rebind', 'tcp:11012', 'tcp:11012']);
  assert.deepEqual(androidForwardRemovalArguments(), ['forward', '--remove', 'tcp:11012']);
  assert.deepEqual(iosProxyArguments('device'), ['--udid', 'device', '--source', '127.0.0.1', '11011:11011']);
});

test('Vite receives only the selected loopback backend origin', () => {
  assert.equal(viteEnvironment('macos', { KEEP: 'yes' }).FLOAT_VIEW_DEV_BACKEND_ORIGIN, 'http://127.0.0.1:11010');
  assert.equal(viteEnvironment('macos', { KEEP: 'yes' }).KEEP, 'yes');
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
    'fail.acheron.floatview/fail.acheron.floatview.MainActivity',
  ]);
  const multiple = 'List of devices attached\none\tdevice\ntwo\tdevice\n';
  assert.throws(() => selectAndroidDeviceFromList(multiple), /Pass an Android target/);
  assert.equal(selectAndroidDeviceFromList(multiple, 'two'), 'two');
});

test('iOS development prefers a booted simulator, forwards logs, and uses same-number iproxy', () => {
  const devices = {
    devices: {
      'iOS 26': [
        { udid: 'shutdown', state: 'Shutdown' },
        { udid: 'booted', state: 'Booted' },
      ],
    },
  };
  assert.equal(selectIOSSimulatorFromList(devices), 'booted');
  assert.deepEqual(iosProxyArguments('physical'), ['--udid', 'physical', '--source', '127.0.0.1', '11011:11011']);
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
    'fail.acheron.float-view',
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
    'fail.acheron.float-view',
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
