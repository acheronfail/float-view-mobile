import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import type { PhysicalTarget, Target } from '../src/suite.ts';

export const ANDROID_BACKEND_PORT = 11012;
const hostPort = ANDROID_BACKEND_PORT;
const devicePort = ANDROID_BACKEND_PORT;
const applicationId = 'fail.acheron.floatview';
const activity = `${applicationId}/${applicationId}.ui.MainActivity`;

export const androidTarget: Target = makeAndroidTarget();

export function androidPhysicalTarget(deviceSerial: string): PhysicalTarget {
  if (!deviceSerial) throw new Error('Android ADB serial is required');
  const root = fileURLToPath(new URL('../../', import.meta.url));
  let prepared = false;
  const prepareSuite = async (report: (message: string) => void = () => undefined) => {
    if (prepared) return;
    assertPhysicalDevice(deviceSerial);
    report('Android: building application...');
    execFileSync(`${root}/backends/android/gradlew`, androidBuildArguments(root), {
      stdio: 'inherit',
    });
    report('Android: application built');
    report('Android: installing application...');
    adb(deviceSerial, 'install', '-r', `${root}/backends/android/app/build/outputs/apk/debug/app-debug.apk`);
    report('Android: application installed');
    const apiLevel = androidApiLevel(deviceSerial);
    for (const permission of androidRuntimePermissions(apiLevel)) {
      adb(deviceSerial, 'shell', 'pm', 'grant', applicationId, permission);
    }
    adb(deviceSerial, 'forward', `tcp:${hostPort}`, `tcp:${devicePort}`);
    prepared = true;
    report('Android: permissions and loopback forwarding ready');
  };
  const prepareLocation = async (report: (message: string) => void = () => undefined) => {
    assertPhysicalDevice(deviceSerial);
    const apiLevel = androidApiLevel(deviceSerial);
    for (const permission of androidLocationRuntimePermissions(apiLevel)) {
      adb(deviceSerial, 'shell', 'pm', 'grant', applicationId, permission);
    }
    report('Android: location permissions ready');
  };
  return {
    name: 'android-physical',
    platform: 'android',
    productionBluetooth: true,
    prepareSuite,
    prepareLocation,
    async start() {
      await prepareSuite();
      adb(deviceSerial, 'shell', 'am', 'force-stop', applicationId);
      adb(deviceSerial, 'shell', 'am', 'force-stop', 'com.android.intentresolver');
      launch(deviceSerial);
      return bootstrap();
    },
    endpoint: bootstrap,
    async background() {
      adb(deviceSerial, 'shell', 'input', 'keyevent', 'KEYCODE_HOME');
      await waitForAndroidBackground(deviceSerial);
    },
    async foreground() {
      launch(deviceSerial, false);
    },
    async stop() {
      await dismissAndroidExportChooser(deviceSerial);
      adbBestEffort(deviceSerial, 'shell', 'am', 'force-stop', applicationId);
    },
    async finishSuite(report) {
      report('Android: stopping application...');
      adbBestEffort(deviceSerial, 'shell', 'am', 'force-stop', applicationId);
      report('Android: removing loopback forwarding...');
      adbBestEffort(deviceSerial, 'forward', '--remove', `tcp:${hostPort}`);
      prepared = false;
      report('Android: cleanup complete');
    },
  };
}

function makeAndroidTarget(configuredSerial?: string): Target {
  let serial = '';
  return {
    name: 'android',
    async start() {
      serial = configuredSerial ?? process.env.FLOAT_VIEW_ANDROID_EMULATOR ?? findEmulator();
      const root = fileURLToPath(new URL('../../', import.meta.url));
      execFileSync(`${root}/backends/android/gradlew`, androidBuildArguments(root), {
        stdio: 'inherit',
      });
      adb(serial, 'install', '-r', `${root}/backends/android/app/build/outputs/apk/debug/app-debug.apk`);
      adb(serial, 'shell', 'pm', 'clear', applicationId);
      const apiLevel = androidApiLevel(serial);
      for (const permission of androidRuntimePermissions(apiLevel)) {
        adb(serial, 'shell', 'pm', 'grant', applicationId, permission);
      }
      adb(serial, 'forward', `tcp:${hostPort}`, `tcp:${devicePort}`);
      adb(serial, 'shell', 'am', 'force-stop', 'com.android.intentresolver');
      launch(serial);
      return bootstrap();
    },
    endpoint: bootstrap,
    async background() {
      adb(serial, 'shell', 'input', 'keyevent', 'KEYCODE_HOME');
      await waitForAndroidBackground(serial);
    },
    async foreground() {
      launch(serial, false);
    },
    async recreateFrontend() {
      adb(serial, 'shell', 'am', 'start', '-W', '-f', '0x10008000', '-n', activity);
    },
    async stop() {
      if (!serial) return;
      adbBestEffort(serial, 'shell', 'am', 'force-stop', applicationId);
      adbBestEffort(serial, 'forward', '--remove', `tcp:${hostPort}`);
      serial = '';
    },
  };
}

export function androidRuntimePermissions(apiLevel: number) {
  if (apiLevel >= 31) {
    return ['android.permission.BLUETOOTH_SCAN', 'android.permission.BLUETOOTH_CONNECT'];
  }
  if (apiLevel >= 23) return ['android.permission.ACCESS_FINE_LOCATION'];
  return [];
}

export function androidLocationRuntimePermissions(apiLevel: number) {
  const permissions = [...androidRuntimePermissions(apiLevel)];
  if (apiLevel >= 23) permissions.push('android.permission.ACCESS_FINE_LOCATION');
  if (apiLevel >= 29) permissions.push('android.permission.ACCESS_BACKGROUND_LOCATION');
  return [...new Set(permissions)];
}

function adb(serial: string, ...args: string[]) {
  return execFileSync('adb', ['-s', serial, ...args], {
    stdio: 'inherit',
    encoding: 'utf8',
  });
}

function adbOutput(serial: string, ...args: string[]) {
  return execFileSync('adb', ['-s', serial, ...args], {
    encoding: 'utf8',
  }).trim();
}

function adbBestEffort(serial: string, ...args: string[]) {
  try {
    execFileSync('adb', ['-s', serial, ...args], { stdio: 'ignore' });
  } catch {}
}

async function dismissAndroidExportChooser(serial: string) {
  // Older Android hosts the chooser in android:ui, not com.android.intentresolver.
  // Dismiss it before killing its FileProvider to avoid a provider-start timeout
  // terminating the next physical scenario's process.
  const deadline = performance.now() + 5_000;
  while (performance.now() < deadline) {
    const activities = adbOutput(serial, 'shell', 'dumpsys', 'activity', 'activities');
    if (!androidExportChooserIsResumed(activities)) return;
    adb(serial, 'shell', 'input', 'keyevent', 'KEYCODE_BACK');
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error('Android export chooser did not close');
}

export function androidExportChooserIsResumed(activities: string) {
  return /(?:mResumedActivity|topResumedActivity)[^\n]*(?:ChooserActivity|ResolverActivity)/.test(activities);
}

async function waitForAndroidBackground(serial: string) {
  const deadline = performance.now() + 5_000;
  while (performance.now() < deadline) {
    const activities = adbOutput(serial, 'shell', 'dumpsys', 'activity', 'activities');
    if (androidActivityIsStopped(activities)) return;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error('Android application did not enter the background');
}

export function androidActivityIsStopped(activities: string) {
  const marker = `packageName=${applicationId}`;
  let offset = 0;
  while ((offset = activities.indexOf(marker, offset)) >= 0) {
    const state = /\n\s*state=([A-Z_]+) stopped=(true|false)/.exec(activities.slice(offset));
    if (state?.[1] !== 'STOPPED' || state[2] !== 'true') return false;
    offset += marker.length;
  }
  return true;
}

function launch(serial: string, stopFirst = true) {
  adb(serial, ...androidLaunchArguments(stopFirst));
}

export function androidBuildArguments(root: string) {
  return ['-p', `${root}/backends/android`, 'assembleDebug'];
}

export function androidStorageResetArguments() {
  return ['shell', 'run-as', applicationId, 'rm', '-rf', 'files/float-view'];
}

export function androidLaunchArguments(stopFirst = true) {
  return [
    'shell',
    'am',
    'start',
    '-W',
    ...(stopFirst ? ['-S', '--activity-clear-task'] : []),
    '-a',
    'android.intent.action.MAIN',
    '-c',
    'android.intent.category.LAUNCHER',
    '-n',
    activity,
  ];
}

function assertPhysicalDevice(serial: string) {
  const value = execFileSync('adb', ['-s', serial, 'shell', 'getprop', 'ro.kernel.qemu'], {
    encoding: 'utf8',
  }).trim();
  if (value === '1') throw new Error('Physical Android conformance cannot target an emulator');
}

function androidApiLevel(serial: string) {
  const value = Number(adbOutput(serial, 'shell', 'getprop', 'ro.build.version.sdk'));
  if (!Number.isInteger(value) || value < 1) {
    throw new Error(`Invalid Android API level: ${value}`);
  }
  return value;
}

function findEmulator(): string {
  const output = execFileSync('adb', ['devices'], { encoding: 'utf8' });
  const value = output
    .split('\n')
    .map((line) => line.split(/\s+/))
    .find(([name, state]) => name?.startsWith('emulator-') && state === 'device')?.[0];
  if (!value) throw new Error('Start an Android Emulator or set FLOAT_VIEW_ANDROID_EMULATOR');
  return value;
}

async function bootstrap() {
  const forwardedOrigin = `http://127.0.0.1:${hostPort}`;
  for (let attempt = 0; attempt < 100; attempt += 1) {
    try {
      const response = await fetch(`${forwardedOrigin}/api/bootstrap`);
      if (response.ok) {
        const value = (await response.json()) as {
          webSocketUrl: string;
          sessionToken: string;
        };
        const url = new URL(value.webSocketUrl);
        url.hostname = '127.0.0.1';
        url.port = String(hostPort);
        url.searchParams.set('token', value.sessionToken);
        return {
          url: url.toString(),
          origin: `http://127.0.0.1:${devicePort}`,
        };
      }
    } catch {}
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error('Android backend did not become ready');
}
