import { unreachableFixtureEnvironment } from '../fixture/selection.ts';
import { mkdtempSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { startFixtureRelay, type FixtureRelay } from '../fixture/relay.ts';
import { observeSpeechReplay, type SpeechReplayOptions } from './speech-replay.ts';
import type { PhysicalTarget, Target } from '../src/suite.ts';

export const MACOS_BACKEND_PORT = 11010;

export const macosTarget: Target = makeMacosTarget(false);
export const macosPhysicalTarget: PhysicalTarget = {
  ...makeMacosTarget(true),
  name: 'macos-physical',
  platform: 'macos',
  productionBluetooth: true,
};

export function makeMacosTarget(
  physical: boolean,
  replay?: SpeechReplayOptions
): Target & Partial<Pick<PhysicalTarget, 'finishSuite' | 'verifyFilePresentation'>> {
  if (physical && replay) throw new Error('Speech replay is software-only');
  let child: ChildProcess | undefined;
  let stopObserving: (() => void) | undefined;
  let fixture: FixtureRelay | undefined;
  let logRoot: string | undefined;
  let uiRoot: string | undefined;
  let currentEndpoint: { url: string; origin: string } | undefined;
  return {
    name: physical ? 'macos-physical' : 'macos',
    async start() {
      const root = fileURLToPath(new URL('../../', import.meta.url));
      let executable = process.env.FLOAT_VIEW_MACOS_EXECUTABLE ?? buildMacosExecutable(root, physical);
      if (physical) {
        uiRoot ??= mkdtempSync(join(tmpdir(), 'float-view-macos-ui-'));
        const bundle = join(uiRoot, 'FloatViewConformance.app');
        execFileSync('bash', [join(root, 'backends/apple/package-macos.sh'), executable, bundle], {
          stdio: 'inherit',
        });
        executable = join(bundle, 'Contents', 'MacOS', 'FloatViewMac');
        process.stdout.write(`  macOS UI host: ${bundle}\n`);
      }
      logRoot ??= mkdtempSync(join(tmpdir(), `float-view-macos-${physical ? 'physical' : 'software'}-`));
      const env: NodeJS.ProcessEnv = {
        ...process.env,
        FLOAT_VIEW_FRONTEND_HTML: `${root}/frontend/build/index.html`,
        FLOAT_VIEW_LOG_ROOT: logRoot,
        FLOAT_VIEW_CONFORMANCE_PORT: '0',
      };
      if (physical) Object.assign(env, unreachableFixtureEnvironment);
      if (!physical) {
        fixture = await startFixtureRelay(replay);
        Object.assign(env, {
          FLOAT_VIEW_FIXTURE_ENDPOINT: fixture.endpoint,
          FLOAT_VIEW_FIXTURE_TOKEN: fixture.token,
          FLOAT_VIEW_FIXTURE_RUN: fixture.run,
          FLOAT_VIEW_FIXTURE_SESSION: fixture.session,
        });
      }
      child = spawn(executable, macosLaunchArguments(physical), {
        cwd: root,
        env,
        stdio: ['ignore', 'pipe', replay ? 'pipe' : 'inherit'],
      });
      if (replay) stopObserving = observeSpeechReplay(child.stderr!, 'FLOAT_VIEW_SPEECH_REPLAY ', replay.onSpeech);
      const readiness = await firstLine(child.stdout!);
      const value = JSON.parse(readiness) as {
        webSocketUrl: string;
        sessionToken: string;
        origin: string;
      };
      currentEndpoint = {
        url: `${value.webSocketUrl}?token=${encodeURIComponent(value.sessionToken)}`,
        origin: value.origin,
      };
      return currentEndpoint;
    },
    async loseFixture() {
      await fixture?.close();
    },
    async endpoint() {
      if (!currentEndpoint) throw new Error('macOS backend is not running');
      return currentEndpoint;
    },
    async stop() {
      stopObserving?.();
      stopObserving = undefined;
      child?.kill('SIGTERM');
      await fixture?.close();
      fixture = undefined;
      child = undefined;
      currentEndpoint = undefined;
      if (!physical && logRoot) {
        rmSync(logRoot, { recursive: true, force: true });
        logRoot = undefined;
      }
    },
    async verifyFilePresentation() {
      if (!physical || !child?.pid) throw new Error('macOS physical host is not running');
      process.stdout.write('  Inspect and dismiss the native Save panel, then press Enter to record the checkpoint.\n');
      await new Promise<void>((resolve, reject) => {
        const finish = (error?: Error) => {
          clearTimeout(timeout);
          process.stdin.off('data', acknowledged);
          process.stdin.pause();
          if (error) reject(error);
          else resolve();
        };
        const acknowledged = (data: Buffer) =>
          finish(data.toString().trim() ? new Error('Native panel checkpoint was not confirmed') : undefined);
        const timeout = setTimeout(
          () => finish(new Error('Native panel checkpoint was not confirmed within 120 seconds')),
          120_000
        );
        process.stdin.once('data', acknowledged);
        process.stdin.resume();
      });
      const deadline = performance.now() + 5_000;
      while (performance.now() < deadline) {
        if (logRoot && readdirSync(join(logRoot, 'Exports')).length === 0) return;
        await new Promise((resolve) => setTimeout(resolve, 50));
      }
      throw new Error('Dismissed native presentation retained its staged file lease');
    },
    async finishSuite() {
      if (logRoot) rmSync(logRoot, { recursive: true, force: true });
      logRoot = undefined;
      if (uiRoot) rmSync(uiRoot, { recursive: true, force: true });
      uiRoot = undefined;
    },
  };
}

export function macosLaunchArguments(physical = false) {
  return [physical ? '--browser-host' : '--test-host'];
}

function buildMacosExecutable(root: string, physical: boolean) {
  const cwd = `${root}/backends/apple`;
  const scratch = physical ? '/tmp/float-view-swift-build' : '/tmp/float-view-swift-conformance';
  const flags = physical ? [] : ['-Xswiftc', '-DFLOAT_VIEW_CONFORMANCE'];
  execFileSync('just', ['apple-rust-runtime', 'Debug'], { cwd: root, stdio: 'inherit' });
  const binaryRoot = execFileSync('swift', ['build', '--scratch-path', scratch, '--show-bin-path'], {
    cwd,
    encoding: 'utf8',
  }).trim();
  // SwiftPM does not model externally linked static archives as inputs. Remove
  // only the disposable executable so a changed Rust archive is always relinked.
  rmSync(join(binaryRoot, 'FloatViewMac'), { force: true });
  execFileSync('swift', ['build', '--scratch-path', scratch, '--product', 'FloatViewMac', ...flags], {
    cwd,
    stdio: 'inherit',
  });
  return join(binaryRoot, 'FloatViewMac');
}

function firstLine(stream: NodeJS.ReadableStream, timeoutMs = 10_000): Promise<string> {
  return new Promise((resolve, reject) => {
    let buffer = '';
    const timeout = setTimeout(() => reject(new Error('macOS backend did not become ready')), timeoutMs);
    stream.on('data', (chunk: Buffer) => {
      buffer += chunk.toString();
      const newline = buffer.indexOf('\n');
      if (newline >= 0) {
        clearTimeout(timeout);
        resolve(buffer.slice(0, newline));
      }
    });
  });
}
