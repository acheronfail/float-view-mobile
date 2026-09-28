import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { join } from 'node:path';

const forbidden =
  /FixtureBluetoothAdapter|SoftwareVescPeripheral|RefloatConfigurationFixtures|FLOAT_VIEW_FIXTURE_ENDPOINT|fixture-run\.json/;
const inspect = (path: string) =>
  assert.ok(!forbidden.test(readFileSync(path).toString('latin1')), `Simulation leaked into ${path}`);
for (const [scratch, configuration] of [
  ['/tmp/float-view-swift-build', 'debug'],
  ['/tmp/float-view-swift-release', 'release'],
]) {
  const root = execFileSync('swift', ['build', '--scratch-path', scratch!, '-c', configuration!, '--show-bin-path'], {
    cwd: '../backends/apple',
    encoding: 'utf8',
  }).trim();
  inspect(join(root, 'FloatViewMac'));
}
for (const path of [
  '/tmp/float-view-ios-release/Build/Products/Release-iphonesimulator/float-view.app',
  '/tmp/float-view-ios-device/Build/Products/Debug-iphoneos/float-view.app',
  '/tmp/float-view-ios/Build/Products/Debug-iphonesimulator/float-view.app',
]) {
  const names = readdirSync(path);
  assert.ok(!names.some((name) => /schemas|values|Fixtures/.test(name)), `Fixture assets leaked into ${path}`);
  for (const name of names) if (name === 'float-view' || name.endsWith('.dylib')) inspect(join(path, name));
}
for (const variant of ['debug', 'release']) {
  const apk = `../backends/android/app/build/outputs/apk/${variant}/app-${variant}${variant === 'release' ? '-unsigned' : ''}.apk`;
  const names = execFileSync('unzip', ['-Z1', apk], { encoding: 'utf8' }).trim().split('\n');
  assert.ok(
    !names.some((name) => /assets\/(refloat|vesc|connection|session-)/.test(name)),
    'Native fixture assets leaked into APK'
  );
  for (const name of names.filter((name) => name.endsWith('.dex'))) {
    const bytes = execFileSync('unzip', ['-p', apk, name], { maxBuffer: 32 * 1024 * 1024 });
    assert.ok(!forbidden.test(bytes.toString('latin1')), `Simulation leaked into ${apk}/${name}`);
  }
}
const files = (path: string): string[] =>
  readdirSync(path).flatMap((name) => {
    const child = join(path, name);
    return statSync(child).isDirectory() ? files(child) : [child];
  });
for (const path of files('../backends/apple/Sources').concat(files('../backends/android/app/src'))) {
  if (!/\.(swift|kt)$/.test(path)) continue;
  const value = readFileSync(path, 'utf8');
  assert.ok(
    !/SoftwareVescPeripheral|SoftwareBluetoothAdapter|isEmulator\(/.test(value),
    `Superseded simulator: ${path}`
  );
  if (path.endsWith('FixtureBluetoothAdapter.swift') || path.endsWith('FixtureBluetoothAdapter.kt'))
    assert.ok(
      !/software-vesc|THOR300|VBMS|crc16|Refloat|encodeFrame|decodeFrame/.test(value),
      `Protocol model in native transport: ${path}`
    );
}
console.log('Simulation exclusion passed: ordinary macOS/iOS/Android, Release, physical iOS, native source ownership');
