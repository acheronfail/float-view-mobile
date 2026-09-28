import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import {
  androidActivityIsStopped,
  ANDROID_BACKEND_PORT,
  androidBuildArguments,
  androidLaunchArguments,
  androidLocationRuntimePermissions,
  androidPhysicalTarget,
  androidRuntimePermissions,
  androidStorageResetArguments,
} from '../targets/android.ts';
import {
  iosPhysicalBuildArguments,
  IOS_BACKEND_PORT,
  iosDeviceIsPreparingFailure,
  iosPhysicalLaunchArguments,
  iosPhysicalProcessIdentifier,
  iosPhysicalTerminateArguments,
  iosPhysicalTarget,
} from '../targets/ios.ts';
import { MACOS_BACKEND_PORT, macosLaunchArguments, macosPhysicalTarget } from '../targets/macos.ts';
import { MOCK_BACKEND_PORT } from '../targets/mock.ts';

test('physical targets validate identity before executing host commands', () => {
  assert.throws(() => androidPhysicalTarget(''), /ADB serial is required/);
  assert.throws(() => iosPhysicalTarget(''), /device ID is required/);
  assert.equal(androidPhysicalTarget('serial').productionBluetooth, true);
  assert.equal(iosPhysicalTarget('device').productionBluetooth, true);
  assert.equal(macosPhysicalTarget.productionBluetooth, true);
});

test('target ports use the stable platform registry', () => {
  assert.equal(MACOS_BACKEND_PORT, 11010);
  assert.equal(IOS_BACKEND_PORT, 11011);
  assert.equal(ANDROID_BACKEND_PORT, 11012);
  assert.equal(MOCK_BACKEND_PORT, 11003);
});

test('physical target command construction uses normal Debug app paths', () => {
  assert.deepEqual(androidBuildArguments('/repo'), ['-p', '/repo/backends/android', 'assembleDebug']);
  assert.deepEqual(androidLaunchArguments(), [
    'shell',
    'am',
    'start',
    '-W',
    '-S',
    '--activity-clear-task',
    '-a',
    'android.intent.action.MAIN',
    '-c',
    'android.intent.category.LAUNCHER',
    '-n',
    'fail.acheron.floatview/fail.acheron.floatview.ui.MainActivity',
  ]);
  assert.deepEqual(androidStorageResetArguments(), [
    'shell',
    'run-as',
    'fail.acheron.floatview',
    'rm',
    '-rf',
    'files/float-view',
  ]);
  assert.deepEqual(iosPhysicalBuildArguments('/repo/app.xcodeproj', 'platform=iOS,id=device', '/tmp/dd'), [
    '-project',
    '/repo/app.xcodeproj',
    '-scheme',
    'float-view',
    '-configuration',
    'Debug',
    '-destination',
    'platform=iOS,id=device',
    '-destination-timeout',
    '30',
    '-derivedDataPath',
    '/tmp/dd',
    '-allowProvisioningUpdates',
    'build-for-testing',
  ]);
  assert.deepEqual(iosPhysicalLaunchArguments('device', '/tmp/launch.json'), [
    'devicectl',
    'device',
    'process',
    'launch',
    '--device',
    'device',
    '--environment-variables',
    '{"FLOAT_VIEW_CONFORMANCE_PORT":"11011","FLOAT_VIEW_CONFORMANCE_RESET":"1"}',
    '--terminate-existing',
    '--quiet',
    '--json-output',
    '/tmp/launch.json',
    'fail.acheron.float-view',
  ]);
  assert.deepEqual(iosPhysicalLaunchArguments('device', '/tmp/launch.json', false), [
    'devicectl',
    'device',
    'process',
    'launch',
    '--device',
    'device',
    '--environment-variables',
    '{"FLOAT_VIEW_CONFORMANCE_PORT":"11011"}',
    '--terminate-existing',
    '--quiet',
    '--json-output',
    '/tmp/launch.json',
    'fail.acheron.float-view',
  ]);
  assert.deepEqual(iosPhysicalTerminateArguments('device', 42), [
    'devicectl',
    'device',
    'process',
    'terminate',
    '--device',
    'device',
    '--pid',
    '42',
    '--quiet',
  ]);
  assert.equal(
    iosPhysicalProcessIdentifier({
      result: { process: { processIdentifier: 42 } },
    }),
    42
  );
  assert.throws(() => iosPhysicalProcessIdentifier({ result: {} }), /no valid application PID/);
  assert.equal(
    iosDeviceIsPreparingFailure(new Error('Timed out waiting for all destinations\nDevice is busy (Preparing iPhone)')),
    true
  );
  assert.equal(iosDeviceIsPreparingFailure(new Error('Signing certificate is invalid')), false);
  assert.deepEqual(macosLaunchArguments(), ['--test-host']);
  assert.deepEqual(macosLaunchArguments(true), ['--browser-host']);
});

test('Android physical permissions follow the device API level', () => {
  assert.deepEqual(androidRuntimePermissions(22), []);
  assert.deepEqual(androidRuntimePermissions(23), ['android.permission.ACCESS_FINE_LOCATION']);
  assert.deepEqual(androidRuntimePermissions(30), ['android.permission.ACCESS_FINE_LOCATION']);
  assert.deepEqual(androidRuntimePermissions(31), [
    'android.permission.BLUETOOTH_SCAN',
    'android.permission.BLUETOOTH_CONNECT',
  ]);
});

test('Android location preparation grants foreground and background capability where supported', () => {
  assert.deepEqual(androidLocationRuntimePermissions(22), []);
  assert.deepEqual(androidLocationRuntimePermissions(28), ['android.permission.ACCESS_FINE_LOCATION']);
  assert.deepEqual(androidLocationRuntimePermissions(30), [
    'android.permission.ACCESS_FINE_LOCATION',
    'android.permission.ACCESS_BACKGROUND_LOCATION',
  ]);
  assert.deepEqual(androidLocationRuntimePermissions(31), [
    'android.permission.BLUETOOTH_SCAN',
    'android.permission.BLUETOOTH_CONNECT',
    'android.permission.ACCESS_FINE_LOCATION',
    'android.permission.ACCESS_BACKGROUND_LOCATION',
  ]);
});

test('Android background lifecycle waits for the Activity to stop', () => {
  assert.equal(
    androidActivityIsStopped(
      'packageName=fail.acheron.floatview\nstate=STOPPED stopped=true\n' +
        'packageName=fail.acheron.floatview\nstate=PAUSED stopped=false'
    ),
    false
  );
  assert.equal(androidActivityIsStopped('packageName=fail.acheron.floatview\nstate=STOPPED stopped=true'), true);
  assert.equal(androidActivityIsStopped('no Float View activity'), true);
});

test('physical CLI fails before touching hardware when required arguments are absent', () => {
  const result = spawnSync(process.execPath, ['--strip-types', 'src/run.ts', 'physical', 'android', '/dev/null'], {
    cwd: new URL('../', import.meta.url),
    encoding: 'utf8',
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Android ADB serial is required/);
});

test('Android export cleanup recognizes only resumed document pickers', async () => {
  const { androidDocumentPickerIsResumed } = await import('../targets/android.ts');
  assert.equal(
    androidDocumentPickerIsResumed(
      'mResumedActivity: ActivityRecord{ u0 com.android.documentsui/.picker.PickActivity t1}'
    ),
    true
  );
  assert.equal(
    androidDocumentPickerIsResumed(
      'topResumedActivity=ActivityRecord{ u0 com.google.android.documentsui/.DocumentsActivity t1}'
    ),
    true
  );
  assert.equal(
    androidDocumentPickerIsResumed(
      'mResumedActivity: ActivityRecord{ u0 fail.acheron.floatview/.ui.MainActivity t1}\nHist #1: DocumentsActivity'
    ),
    false
  );
});
