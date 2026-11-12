import { execFileSync, spawn } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createServer } from 'node:net';

export const nativeApplicationId = 'fail.acheron.floatview';

export function assignedPort(value: string): number {
  if (!/^\d+$/.test(value.trim())) throw new Error('Invalid native readiness port');
  const port = Number(value.trim());
  if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error('Invalid native readiness port');
  return port;
}

export function androidDevicePort(serial: string) {
  return assignedPort(
    execFileSync('adb', ['-s', serial, 'shell', 'run-as', nativeApplicationId, 'cat', 'files/backend-port'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
      timeout: 3_000,
    })
  );
}

export function iosSimulatorPort(device: string) {
  const container = execFileSync('xcrun', ['simctl', 'get_app_container', device, nativeApplicationId, 'data'], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore'],
    timeout: 3_000,
  }).trim();
  return assignedPort(readFileSync(join(container, 'Library/Caches/backend-port'), 'utf8'));
}

export function iosDevicePort(device: string) {
  const scratch = mkdtempSync(join(tmpdir(), 'float-view-port-'));
  try {
    const destination = join(scratch, 'backend-port');
    execFileSync(
      'xcrun',
      [
        'devicectl',
        'device',
        'copy',
        'from',
        '--device',
        device,
        '--domain-type',
        'appDataContainer',
        '--domain-identifier',
        nativeApplicationId,
        '--source',
        'Library/Caches/backend-port',
        '--destination',
        destination,
        '--quiet',
        '--timeout',
        '5',
      ],
      {
        stdio: 'ignore',
        timeout: 7_000,
      }
    );
    return assignedPort(readFileSync(destination, 'utf8'));
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

export async function forwardedBootstrap(host: number, device: number) {
  const response = await fetch(`http://127.0.0.1:${host}/api/bootstrap`, { signal: AbortSignal.timeout(1_000) });
  if (!response.ok) throw new Error(`Native bootstrap failed: ${response.status}`);
  const value = (await response.json()) as { webSocketUrl: string; sessionToken: string };
  const url = new URL(value.webSocketUrl);
  if (url.protocol !== 'ws:' || url.hostname !== '127.0.0.1' || Number(url.port) !== device || !value.sessionToken) {
    throw new Error('Native bootstrap does not match discovered endpoint');
  }
  const origin = `http://127.0.0.1:${device}`;
  url.port = String(host);
  url.searchParams.set('token', value.sessionToken);
  return { url: url.toString(), origin };
}

/** Each target owns exactly the forwarding rule allocated by ADB tcp:0. */
export function androidEndpoint(serial: () => string) {
  let host = 0;
  let device = 0;
  const close = () => {
    if (!host) return;
    execFileSync('adb', ['-s', serial(), 'forward', '--remove', `tcp:${host}`], { stdio: 'ignore', timeout: 3_000 });
    host = 0;
    device = 0;
  };
  return {
    close,
    async endpoint() {
      const deadline = performance.now() + 30_000;
      while (performance.now() < deadline) {
        try {
          const next = androidDevicePort(serial());
          if (next !== device) {
            close();
            host = assignedPort(
              execFileSync('adb', ['-s', serial(), 'forward', 'tcp:0', `tcp:${next}`], {
                encoding: 'utf8',
                timeout: 3_000,
              })
            );
            device = next;
          }
          return await forwardedBootstrap(host, device);
        } catch {}
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
      throw new Error('Android backend did not become ready at its discovered port');
    },
  };
}

// iproxy cannot inherit a reserved listener. Its caller must monitor bind failure.
export async function availableHostPort(): Promise<number> {
  const listener = createServer();
  await new Promise<void>((resolve, reject) => {
    listener.once('error', reject);
    listener.listen(0, '127.0.0.1', resolve);
  });
  const address = listener.address();
  if (!address || typeof address === 'string') throw new Error('Missing assigned host port');
  await new Promise<void>((resolve, reject) => listener.close((error) => (error ? reject(error) : resolve())));
  return address.port;
}

/** Discover afresh after each process launch; recreate only this target's proxy. */
export function iosEndpoint(
  deviceId: () => string,
  simulator: boolean,
  observeProxy?: (child: import('node:child_process').ChildProcess) => void
) {
  let host = 0;
  let device = 0;
  let proxy: import('node:child_process').ChildProcess | undefined;
  let proxyFailure: Error | undefined;
  const close = async () => {
    const child = proxy;
    proxy = undefined;
    if (child?.pid && child.exitCode === null && child.signalCode === null) {
      await new Promise<void>((resolve) => {
        const timer = setTimeout(() => child.kill('SIGKILL'), 2_000);
        child.once('close', () => {
          clearTimeout(timer);
          resolve();
        });
        child.kill('SIGTERM');
      });
    }
    host = 0;
    device = 0;
    proxyFailure = undefined;
  };
  return {
    close,
    async endpoint() {
      const deadline = performance.now() + 30_000;
      while (performance.now() < deadline) {
        if (proxyFailure) throw proxyFailure;
        try {
          const next = simulator ? iosSimulatorPort(deviceId()) : iosDevicePort(deviceId());
          if (next !== device) {
            await close();
            device = next;
            host = simulator ? next : await availableHostPort();
            if (!simulator) {
              const child = spawn('iproxy', ['--udid', deviceId(), '--source', '127.0.0.1', `${host}:${device}`], {
                stdio: ['ignore', 'ignore', 'pipe'],
              });
              proxy = child;
              observeProxy?.(child);
              let diagnostics = '';
              child.stderr?.on('data', (chunk: Buffer) => {
                diagnostics = (diagnostics + chunk.toString()).slice(-4096);
              });
              child.once('error', (error) => {
                if (proxy === child) proxyFailure = new Error(`iproxy failed: ${error.message}`);
              });
              child.once('exit', () => {
                if (proxy === child) proxyFailure = new Error(`iproxy exited: ${diagnostics.trim()}`);
              });
            }
          }
          return await forwardedBootstrap(host, device);
        } catch (error) {
          if (proxyFailure) throw proxyFailure;
        }
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
      throw new Error('iOS backend did not become ready at its discovered port');
    },
  };
}
