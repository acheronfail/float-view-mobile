import { copyFileSync, mkdirSync, mkdtempSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import type { PhysicalTarget, Target } from '../src/suite.ts';

export const MACOS_BACKEND_PORT = 11010;

export const macosTarget: Target = makeMacosTarget(false);
export const macosPhysicalTarget: PhysicalTarget = {
  ...makeMacosTarget(true),
  name: 'macos-physical',
  platform: 'macos',
  productionBluetooth: true,
};

function makeMacosTarget(
  physical: boolean
): Target & Partial<Pick<PhysicalTarget, 'finishSuite' | 'verifyFilePresentation'>> {
  let child: ChildProcess | undefined;
  let logRoot: string | undefined;
  let uiRoot: string | undefined;
  let currentEndpoint: { url: string; origin: string } | undefined;
  return {
    name: physical ? 'macos-physical' : 'macos',
    async start() {
      const root = fileURLToPath(new URL('../../', import.meta.url));
      let executable = process.env.FLOAT_VIEW_MACOS_EXECUTABLE ?? buildMacosExecutable(root);
      if (physical) {
        uiRoot ??= mkdtempSync(join(tmpdir(), 'float-view-macos-ui-'));
        const bundle = join(uiRoot, 'FloatViewConformance.app');
        const contents = join(bundle, 'Contents');
        const binary = join(contents, 'MacOS', 'FloatViewMac');
        mkdirSync(join(contents, 'MacOS'), { recursive: true });
        copyFileSync(join(root, 'backends/apple/FloatViewMac-Info.plist'), join(contents, 'Info.plist'));
        rmSync(binary, { force: true });
        copyFileSync(executable, binary);
        executable = binary;
        process.stdout.write(`  macOS UI host: ${bundle}\n`);
      }
      logRoot ??= mkdtempSync(join(tmpdir(), `float-view-macos-${physical ? 'physical' : 'software'}-`));
      const env: NodeJS.ProcessEnv = {
        ...process.env,
        FLOAT_VIEW_FRONTEND_HTML: `${root}/frontend/build/index.html`,
        FLOAT_VIEW_LOG_ROOT: logRoot,
        FLOAT_VIEW_CONFORMANCE_PORT: '0',
      };
      if (physical) {
        delete env.FLOAT_VIEW_SOFTWARE_BACKEND;
      } else {
        env.FLOAT_VIEW_SOFTWARE_BACKEND = '1';
      }
      child = spawn(executable, macosLaunchArguments(physical), {
        cwd: root,
        env,
        stdio: ['ignore', 'pipe', 'inherit'],
      });
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
    async endpoint() {
      if (!currentEndpoint) throw new Error('macOS backend is not running');
      return currentEndpoint;
    },
    async stop() {
      child?.kill('SIGTERM');
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

function buildMacosExecutable(root: string) {
  const cwd = `${root}/backends/apple`;
  const scratch = '/tmp/float-view-swift-build';
  execFileSync('just', ['apple-rust-runtime', 'Debug'], { cwd: root, stdio: 'inherit' });
  const binaryRoot = execFileSync('swift', ['build', '--scratch-path', scratch, '--show-bin-path'], {
    cwd,
    encoding: 'utf8',
  }).trim();
  // SwiftPM does not model externally linked static archives as inputs. Remove
  // only the disposable executable so a changed Rust archive is always relinked.
  rmSync(join(binaryRoot, 'FloatViewMac'), { force: true });
  execFileSync('swift', ['build', '--scratch-path', scratch, '--product', 'FloatViewMac'], {
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
