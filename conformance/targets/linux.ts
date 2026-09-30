import { spawn, execFileSync, type ChildProcess } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'node:net';
import { startFixtureRelay, type FixtureRelay } from '../fixture/relay.ts';
import { unreachableFixtureEnvironment } from '../fixture/selection.ts';
import type { PhysicalTarget, Target } from '../src/suite.ts';

export const LINUX_BACKEND_PORT = 11013;
const origin = `http://127.0.0.1:${LINUX_BACKEND_PORT}`;
export const linuxTarget = makeLinuxTarget(false);
export const linuxPhysicalTarget: PhysicalTarget = {
  ...makeLinuxTarget(true),
  name: 'linux-physical',
  platform: 'linux',
  productionBluetooth: true,
};
function makeLinuxTarget(physical: boolean): Target & Partial<Pick<PhysicalTarget, 'finishSuite'>> {
  let child: ChildProcess | undefined;
  let root: string | undefined;
  let relay: FixtureRelay | undefined;
  let built = false;
  let endpoint: { url: string; origin: string } | undefined;
  const repo = fileURLToPath(new URL('../../', import.meta.url));
  return {
    name: physical ? 'linux-physical' : 'linux',
    async start() {
      // Refuse to attach to an unrelated running app or mutate its data.
      const guard = createServer();
      await new Promise<void>((resolve, reject) => {
        guard.once('error', reject);
        guard.listen(LINUX_BACKEND_PORT, '127.0.0.1', resolve);
      });
      await new Promise<void>((resolve) => guard.close(() => resolve()));
      if (!built) {
        execFileSync(
          'cargo',
          [
            'build',
            '--manifest-path',
            'backends/linux/Cargo.toml',
            '--locked',
            '--offline',
            ...(physical ? [] : ['--features', 'software-bluetooth']),
          ],
          { cwd: repo, stdio: 'inherit' }
        );
        built = true;
      }
      root ??= mkdtempSync(join(tmpdir(), 'float-view-linux-conformance-'));
      const env = { ...process.env, XDG_DATA_HOME: root, ...unreachableFixtureEnvironment };
      if (!physical) {
        relay = await startFixtureRelay();
        Object.assign(env, {
          FLOAT_VIEW_FIXTURE_ENDPOINT: relay.endpoint,
          FLOAT_VIEW_FIXTURE_TOKEN: relay.token,
          FLOAT_VIEW_FIXTURE_RUN: relay.run,
          FLOAT_VIEW_FIXTURE_SESSION: relay.session,
        });
      }
      child = spawn(
        join(repo, 'backends/linux/target/debug/float-view-linux'),
        physical ? [] : ['--software-conformance'],
        { cwd: repo, env, stdio: ['ignore', 'ignore', 'pipe'] }
      );
      let error: Error | undefined;
      child.on('error', (value) => {
        error = value;
      });
      // Bound diagnostics; never echo readiness credentials or protocol payloads.
      let diagnostic = '';
      child.stderr?.on('data', (data: Buffer) => {
        diagnostic = (diagnostic + data.toString()).slice(-4096);
      });
      for (const deadline = Date.now() + 30_000; Date.now() < deadline;) {
        if (error || child.exitCode !== null) throw error ?? new Error(`Linux host exited: ${diagnostic}`);
        try {
          const response = await fetch(`${origin}/api/bootstrap`, { signal: AbortSignal.timeout(500) });
          const bootstrap = (await response.json()) as { webSocketUrl: string; sessionToken: string; platform: string };
          if (bootstrap.platform !== 'linux') throw new Error('Unexpected Linux bootstrap');
          endpoint = { url: `${bootstrap.webSocketUrl}?token=${encodeURIComponent(bootstrap.sessionToken)}`, origin };
          // Bluetooth observations are asynchronous; readiness includes their first observation.
          const { ConformanceClient } = await import('../src/client.ts');
          const client = await ConformanceClient.open(endpoint.url, endpoint.origin);
          try {
            const state = await client.request('state.get', {});
            if (state.bluetooth.radio !== 'unknown') return endpoint;
          } finally {
            client.close();
          }
        } catch {}
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
      throw new Error(`Linux readiness timed out: ${diagnostic}`);
    },
    async endpoint() {
      if (!endpoint) throw new Error('Linux host is not running');
      return endpoint;
    },
    ...(!physical
      ? {
          async loseFixture() {
            await relay?.close();
          },
        }
      : {}),
    async finishSuite() {
      if (root) {
        rmSync(root, { recursive: true, force: true });
        root = undefined;
      }
    },
    async stop() {
      if (child && child.exitCode === null && child.signalCode === null) {
        const owned = child;
        await new Promise<void>((resolve) => {
          const timer = setTimeout(() => owned.kill('SIGKILL'), 12_000);
          owned.once('exit', () => {
            clearTimeout(timer);
            resolve();
          });
          owned.kill('SIGTERM');
        });
      }
      child = undefined;
      endpoint = undefined;
      await relay?.close();
      relay = undefined;
      if (!physical && root) {
        rmSync(root, { recursive: true, force: true });
        root = undefined;
      }
    },
  };
}
