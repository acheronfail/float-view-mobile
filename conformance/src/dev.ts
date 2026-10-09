import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import { platform } from 'node:os';
import { rmSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

export const VITE_PORT = 11002;
export const MOCK_PORT = 11003;
export const MACOS_PORT = 11010;
export const IOS_PORT = 11011;
export const ANDROID_PORT = 11012;
const ANDROID_APPLICATION = 'fail.acheron.floatview';
const ANDROID_ACTIVITY = `${ANDROID_APPLICATION}/${ANDROID_APPLICATION}.ui.MainActivity`;
const IOS_BUNDLE = 'fail.acheron.float-view';
const READY_TIMEOUT_MS = 30_000;

type TargetPlatform = 'mock' | 'macos' | 'ios' | 'android';

export function backendOrigin(targetPlatform: TargetPlatform) {
  switch (targetPlatform) {
    case 'mock':
      return `http://127.0.0.1:${MOCK_PORT}`;
    case 'macos':
      return `http://127.0.0.1:${MACOS_PORT}`;
    case 'ios':
      return `http://127.0.0.1:${IOS_PORT}`;
    case 'android':
      return `http://127.0.0.1:${ANDROID_PORT}`;
  }
}

export function androidForwardArguments() {
  return ['forward', '--no-rebind', `tcp:${ANDROID_PORT}`, `tcp:${ANDROID_PORT}`];
}

export function androidForwardRemovalArguments() {
  return ['forward', '--remove', `tcp:${ANDROID_PORT}`];
}

export function androidInstallArguments(root: string) {
  return ['install', '-r', `${root}/backends/android/app/build/outputs/apk/debug/app-debug.apk`];
}

export function androidLaunchArguments() {
  return ['shell', 'am', 'start', '-W', '-n', ANDROID_ACTIVITY];
}

export function androidLogcatArguments(processId: string) {
  return ['logcat', '--pid', processId, '-v', 'color'];
}

export function iosProxyArguments(device: string) {
  return ['--udid', device, '--source', '127.0.0.1', `${IOS_PORT}:${IOS_PORT}`];
}

export function iosSimulatorBuildArguments(project: string, device: string, derived: string) {
  return [
    '-project',
    project,
    '-scheme',
    'float-view',
    '-sdk',
    'iphonesimulator',
    '-destination',
    `id=${device}`,
    '-derivedDataPath',
    derived,
    'CODE_SIGNING_ALLOWED=NO',
    'build',
  ];
}

export function iosSimulatorConsoleLaunchArguments(device: string) {
  return ['simctl', 'launch', '--terminate-running-process', '--console', device, IOS_BUNDLE];
}

export function iosPhysicalConsoleLaunchArguments(device: string) {
  return [
    'devicectl',
    'device',
    'process',
    'launch',
    '--device',
    device,
    '--terminate-existing',
    '--console',
    IOS_BUNDLE,
  ];
}

export function childExitFailure(label: string, code: number | null, signal: NodeJS.Signals | null) {
  if (code === 0 || signal === 'SIGTERM') return undefined;
  return new Error(`${label} exited${signal ? ` with signal ${signal}` : ` with code ${code ?? 'unknown'}`}`);
}

export function viteEnvironment(
  targetPlatform: TargetPlatform,
  environment: NodeJS.ProcessEnv = process.env
): NodeJS.ProcessEnv {
  return {
    ...environment,
    FLOAT_VIEW_DEV_BACKEND_ORIGIN: backendOrigin(targetPlatform),
  };
}

export async function runDevelopment(targetPlatform: TargetPlatform, target = '', environment = process.env) {
  const root = fileURLToPath(new URL('../../', import.meta.url));
  const children: ChildProcess[] = [];
  const cleanup: Array<() => void | Promise<void>> = [];
  let firstFailure: Error | undefined;

  const track = (child: ChildProcess, label: string) => {
    children.push(child);
    child.once('error', (error) => {
      firstFailure ??= new Error(`${label} failed: ${error.message}`);
    });
    child.once('exit', (code, signal) => {
      firstFailure ??= childExitFailure(label, code, signal);
    });
    return child;
  };

  try {
    if (targetPlatform === 'macos') {
      const executable = buildMacos(root);
      track(
        spawn(executable, ['--browser-host'], {
          cwd: root,
          env: {
            ...environment,
            FLOAT_VIEW_FRONTEND_HTML: `${root}/frontend/build/index.html`,
            FLOAT_VIEW_SQL_ROOT: `${root}/backends/sql`,
          },
          stdio: ['ignore', 'inherit', 'inherit'],
        }),
        `macOS backend on 127.0.0.1:${MACOS_PORT}`
      );
      await waitForBootstrap(backendOrigin(targetPlatform), () => firstFailure);
    } else if (targetPlatform === 'android') {
      const serial = target || selectAndroidDevice();
      execFileSync(`${root}/backends/android/gradlew`, ['-p', `${root}/backends/android`, 'assembleDebug'], {
        stdio: 'inherit',
      });
      adb(serial, ...androidInstallArguments(root));
      adb(serial, 'shell', 'am', 'force-stop', ANDROID_APPLICATION);
      adb(serial, ...androidForwardArguments());
      cleanup.push(() => adbBestEffort(serial, ...androidForwardRemovalArguments()));
      cleanup.push(() => adbBestEffort(serial, 'shell', 'am', 'force-stop', ANDROID_APPLICATION));
      adbBestEffort(serial, 'logcat', '-c');
      adb(serial, ...androidLaunchArguments());
      const processId = androidProcessId(serial);
      console.log(`Streaming Float View logs from Android process ${processId}; press Ctrl-C to stop.`);
      track(
        spawn('adb', ['-s', serial, ...androidLogcatArguments(processId)], {
          stdio: ['ignore', 'inherit', 'inherit'],
        }),
        `Android logcat for process ${processId} on ${serial}`
      );
      await waitForBootstrap(backendOrigin(targetPlatform), () => firstFailure);
    } else if (targetPlatform === 'ios') {
      await startIOS(root, target, track, cleanup, environment);
      await waitForBootstrap(backendOrigin(targetPlatform), () => firstFailure);
    }

    const vite = track(
      spawn('npm', ['run', 'dev'], {
        cwd: `${root}/frontend`,
        env: viteEnvironment(targetPlatform, environment),
        stdio: 'inherit',
      }),
      'Vite'
    );
    await waitForURL(`http://127.0.0.1:${VITE_PORT}/`, () => firstFailure);
    console.log(`Float View ${targetPlatform} development ready: http://127.0.0.1:${VITE_PORT}`);
    if (environment.FLOAT_VIEW_DEV_NO_OPEN !== '1') {
      const hostPlatform = platform();
      if (hostPlatform === 'darwin') {
        execFileSync('open', [`http://127.0.0.1:${VITE_PORT}`], { stdio: 'ignore' });
      } else if (hostPlatform === 'linux') {
        execFileSync('xdg-open', [`http://127.0.0.1:${VITE_PORT}`], { stdio: 'ignore' });
      } else {
        console.warn(`Currently unsupported host platform: ${hostPlatform}!`);
      }
    }
    await waitForStop([vite, ...children.filter((child) => child !== vite)]);
    if (firstFailure) throw firstFailure;
  } finally {
    for (const child of children.reverse()) await stopChild(child);
    for (const action of cleanup.reverse()) {
      try {
        await action();
      } catch {}
    }
  }
}

async function startIOS(
  root: string,
  target: string,
  track: (child: ChildProcess, label: string) => ChildProcess,
  cleanup: Array<() => void | Promise<void>>,
  environment: NodeJS.ProcessEnv
) {
  const device = target || selectIOSSimulator();
  const simulators = iosSimulatorIDs();
  const project = `${root}/backends/apple/ios/float-view.xcodeproj`;
  if (simulators.has(device)) {
    const derived = '/tmp/float-view-ios-dev';
    execBestEffort('xcrun', ['simctl', 'boot', device]);
    execFileSync('xcrun', ['simctl', 'bootstatus', device, '-b'], {
      stdio: 'inherit',
    });
    execFileSync('xcodebuild', iosSimulatorBuildArguments(project, device, derived), {
      stdio: 'inherit',
    });
    execFileSync(
      'xcrun',
      ['simctl', 'install', device, `${derived}/Build/Products/Debug-iphonesimulator/float-view.app`],
      { stdio: 'inherit' }
    );
    track(
      spawn('xcrun', iosSimulatorConsoleLaunchArguments(device), {
        env: {
          ...environment,
          SIMCTL_CHILD_FLOAT_VIEW_CONFORMANCE_PORT: String(IOS_PORT),
        },
        stdio: 'inherit',
      }),
      `iOS Simulator backend console on ${device}`
    );
    cleanup.push(() => execBestEffort('xcrun', ['simctl', 'terminate', device, IOS_BUNDLE]));
    return;
  }

  const derived = '/tmp/float-view-ios-physical-dev';
  execFileSync(
    'xcodebuild',
    [
      '-project',
      project,
      '-scheme',
      'float-view',
      '-configuration',
      'Debug',
      '-destination',
      `platform=iOS,id=${device}`,
      '-destination-timeout',
      '30',
      '-derivedDataPath',
      derived,
      '-allowProvisioningUpdates',
      'build',
    ],
    { stdio: 'inherit' }
  );
  execFileSync(
    'xcrun',
    [
      'devicectl',
      'device',
      'install',
      'app',
      '--device',
      device,
      `${derived}/Build/Products/Debug-iphoneos/float-view.app`,
    ],
    { stdio: 'inherit' }
  );
  const proxy = spawn('iproxy', iosProxyArguments(device), {
    stdio: ['ignore', 'ignore', 'inherit'],
  });
  track(proxy, `iOS iproxy on 127.0.0.1:${IOS_PORT}`);
  track(
    spawn('xcrun', iosPhysicalConsoleLaunchArguments(device), {
      env: environment,
      stdio: 'inherit',
    }),
    `iOS backend console on ${device}`
  );
}

function buildMacos(root: string) {
  const cwd = `${root}/backends/apple`;
  const scratch = '/tmp/float-view-swift-build';
  execFileSync('just', ['apple-rust-runtime', 'Debug'], { cwd: root, stdio: 'inherit' });
  const binaryRoot = execFileSync('swift', ['build', '--scratch-path', scratch, '--show-bin-path'], {
    cwd,
    encoding: 'utf8',
  }).trim();
  // SwiftPM does not track the externally linked Rust archive as a build input.
  rmSync(join(binaryRoot, 'FloatViewMac'), { force: true });
  execFileSync('swift', ['build', '--scratch-path', scratch, '--product', 'FloatViewMac'], {
    cwd,
    stdio: 'inherit',
  });
  return join(binaryRoot, 'FloatViewMac');
}

export function selectAndroidDeviceFromList(output: string, target = '') {
  if (target) return target;
  const devices = output
    .split('\n')
    .slice(1)
    .map((line) => line.trim().split(/\s+/))
    .filter(([, state]) => state === 'device')
    .map(([serial]) => serial);
  const serial = devices[0];
  if (devices.length !== 1 || !serial) {
    throw new Error('Pass an Android target when exactly one connected device is not available');
  }
  return serial;
}

function selectAndroidDevice() {
  return selectAndroidDeviceFromList(execFileSync('adb', ['devices'], { encoding: 'utf8' }));
}

function androidProcessId(serial: string) {
  const processId = execFileSync('adb', ['-s', serial, 'shell', 'pidof', '-s', ANDROID_APPLICATION], {
    encoding: 'utf8',
  }).trim();
  if (!/^\d+$/.test(processId)) throw new Error(`Float View did not start on Android device ${serial}`);
  return processId;
}

function iosSimulatorIDs() {
  const value = JSON.parse(
    execFileSync('xcrun', ['simctl', 'list', 'devices', 'available', '--json'], {
      encoding: 'utf8',
    })
  ) as { devices: Record<string, Array<{ udid: string; state: string }>> };
  return new Set(
    Object.values(value.devices)
      .flat()
      .map(({ udid }) => udid)
  );
}

export function selectIOSSimulatorFromList(value: { devices: Record<string, Array<{ udid: string; state: string }>> }) {
  const devices = Object.values(value.devices).flat();
  return (
    devices.find(({ state }) => state === 'Booted')?.udid ??
    devices[0]?.udid ??
    (() => {
      throw new Error('No iOS Simulator is available');
    })()
  );
}

function selectIOSSimulator() {
  return selectIOSSimulatorFromList(
    JSON.parse(
      execFileSync('xcrun', ['simctl', 'list', 'devices', 'available', '--json'], {
        encoding: 'utf8',
      })
    )
  );
}

function adb(serial: string, ...arguments_: string[]) {
  execFileSync('adb', ['-s', serial, ...arguments_], { stdio: 'inherit' });
}

function adbBestEffort(serial: string, ...arguments_: string[]) {
  execBestEffort('adb', ['-s', serial, ...arguments_]);
}

function execBestEffort(command: string, arguments_: string[]) {
  try {
    execFileSync(command, arguments_, { stdio: 'ignore' });
  } catch {}
}

async function waitForBootstrap(origin: string, failure: () => Error | undefined) {
  await waitForURL(`${origin}/api/bootstrap`, failure);
}

export async function waitForURL(url: string, failure: () => Error | undefined, timeout = READY_TIMEOUT_MS) {
  const deadline = performance.now() + timeout;
  while (performance.now() < deadline) {
    const currentFailure = failure();
    if (currentFailure) throw currentFailure;
    try {
      const response = await fetch(url, { signal: AbortSignal.timeout(1_000) });
      if (response.ok) return;
    } catch {}
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`${url} did not become ready within ${timeout / 1_000}s`);
}

function waitForStop(children: ChildProcess[]) {
  return new Promise<void>((resolve) => {
    const done = () => resolve();
    process.once('SIGINT', done);
    process.once('SIGTERM', done);
    for (const child of children) child.once('exit', done);
  });
}

async function stopChild(child: ChildProcess) {
  if (child.exitCode !== null || child.signalCode !== null) return;
  await new Promise<void>((resolve) => {
    const timer = setTimeout(() => child.kill('SIGKILL'), 2_000);
    child.once('exit', () => {
      clearTimeout(timer);
      resolve();
    });
    child.kill('SIGTERM');
  });
}

const isMain = !!process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (isMain) {
  const targetPlatform = process.argv[2] as TargetPlatform | undefined;
  if (!targetPlatform || !['mock', 'macos', 'ios', 'android'].includes(targetPlatform)) {
    throw new Error('Usage: just dev mock|macos|ios|android [target]');
  }
  await runDevelopment(targetPlatform, process.argv[3] ?? '');
}
