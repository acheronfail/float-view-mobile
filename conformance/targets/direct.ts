import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync, spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import { SoftwarePeripheral } from '../fixture/software.ts';
import type { Target } from '../src/suite.ts';

type Effect = {
  effect: string;
  id?: number;
  delay_ms?: number;
  connection?: boolean;
  generation?: number;
  scan_id?: number;
  bytes?: number[];
};
export function makeDirectTarget(): Target {
  let child: ChildProcessWithoutNullStreams | undefined;
  let root: string | undefined;
  let endpoint: { url: string; origin: string } | undefined;
  let lifetime = 0,
    sequence = 0,
    request = 0;
  const pending = new Map<number, { resolve: (v: unknown) => void; reject: (e: Error) => void }>();
  const timers = new Map<number, { timer: NodeJS.Timeout; connection: boolean }>();
  let polling: NodeJS.Timeout | undefined;
  let busy = false;
  let peripheral = new SoftwarePeripheral();
  function command(command: string, extra: object = {}): Promise<unknown> {
    if (pending.size >= 64) return Promise.reject(new Error('OS relay capacity'));
    const id = ++request;
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
      child!.stdin.write(JSON.stringify({ id, command, ...extra }) + '\n');
    });
  }
  function fact(fact: object) {
    return command('fact', {
      envelope: {
        lifetime,
        sequence: ++sequence,
        monotonic_ms: Math.floor(performance.now()),
        epoch_ms: Date.now(),
        fact,
      },
    });
  }
  async function effect(e: Effect) {
    switch (e.effect) {
      case 'scan':
        await fact({
          fact: 'discovered',
          scan_id: e.scan_id,
          device_id: 'software-vesc',
          name: 'Software VESC',
          rssi: -42,
        });
        break;
      case 'connect':
        await fact({ fact: 'link', generation: e.generation, state: 'ready' });
        break;
      case 'write':
        for (const bytes of peripheral.receive(Uint8Array.from(e.bytes!)))
          await fact({ fact: 'bytes', generation: e.generation, bytes: Array.from(bytes) });
        break;
      case 'arm_timer': {
        const id = e.id!;
        timers.set(id, {
          connection: e.connection!,
          timer: setTimeout(() => {
            timers.delete(id);
            void fact({ fact: 'timer', id }).catch(fail);
          }, e.delay_ms),
        });
        break;
      }
      case 'cancel_timer':
        clearTimeout(timers.get(e.id!)?.timer);
        timers.delete(e.id!);
        break;
      case 'cancel_connection_timers':
        for (const [id, t] of timers)
          if (t.connection) {
            clearTimeout(t.timer);
            timers.delete(id);
          }
        break;
      case 'request_bluetooth_permission':
        await fact({ fact: 'bluetooth', radio: 'poweredOn', scan: 'granted', connect: 'granted' });
        await fact({ fact: 'ui_completed', id: e.id, success: true });
        break;
      case 'request_location_permission':
      case 'open_location_settings':
        await fact({ fact: 'location_capability', authorization: 'unavailable', background: false });
        await fact({ fact: 'ui_completed', id: e.id, success: true });
        break;
      case 'present_file':
        await fact({ fact: 'presentation_finished', id: e.id });
        break;
    }
  }
  function fail(error: unknown) {
    for (const p of pending.values()) p.reject(error instanceof Error ? error : new Error(String(error)));
    pending.clear();
  }
  return {
    name: 'direct',
    async start() {
      const repo = fileURLToPath(new URL('../../', import.meta.url));
      execFileSync(
        'cargo',
        [
          'build',
          '--locked',
          '--offline',
          '--manifest-path',
          join(repo, 'backends/core/Cargo.toml'),
          '-p',
          'float-view-loopback-server',
          '--example',
          'shared-service',
        ],
        { stdio: 'inherit' }
      );
      root = mkdtempSync(join(tmpdir(), 'float-view-direct-'));
      sequence = 0;
      request = 0;
      peripheral = new SoftwarePeripheral();
      child = spawn(join(repo, 'backends/core/target/debug/examples/shared-service'), [root], { stdio: 'pipe' });
      child.stderr.pipe(process.stderr);
      const ready = new Promise<void>((resolve, reject) => {
        const timeout = setTimeout(() => reject(new Error('Direct service readiness timeout')), 10_000);
        createInterface({ input: child!.stdout }).on('line', (line) => {
          const value = JSON.parse(line) as {
            event?: string;
            lifetime: number;
            url: string;
            origin: string;
            id: number;
            result: { Ok?: unknown; Err?: string };
          };
          if (value.event === 'ready') {
            lifetime = value.lifetime;
            endpoint = { url: value.url, origin: value.origin };
            clearTimeout(timeout);
            resolve();
            return;
          }
          const waiter = pending.get(value.id);
          pending.delete(value.id);
          if (value.result.Err) waiter?.reject(new Error(value.result.Err));
          else waiter?.resolve(value.result.Ok);
        });
        child!.on('exit', (code) => {
          if (code) reject(new Error(`Direct service exited ${code}`));
        });
      });
      await ready;
      await fact({ fact: 'bluetooth', radio: 'poweredOn', scan: 'granted', connect: 'granted' });
      await fact({ fact: 'storage', available_bytes: 1_000_000_000 });
      polling = setInterval(() => {
        if (busy) return;
        busy = true;
        void command('drain', {
          envelope: {
            lifetime,
            sequence: ++sequence,
            monotonic_ms: Math.floor(performance.now()),
            epoch_ms: Date.now(),
            fact: { fact: 'clock' },
          },
        })
          .then(async (result) => {
            for (const e of (result as { effects: Effect[] }).effects) await effect(e);
          })
          .catch(fail)
          .finally(() => {
            busy = false;
          });
      }, 5);
      return endpoint!;
    },
    async endpoint() {
      if (!endpoint) throw new Error('Direct service not running');
      return endpoint;
    },
    async stop() {
      clearInterval(polling);
      for (const t of timers.values()) clearTimeout(t.timer);
      timers.clear();
      child?.stdin.end();
      if (child)
        await new Promise<void>((resolve) => {
          child!.once('exit', () => resolve());
          setTimeout(() => {
            child?.kill();
            resolve();
          }, 5_000).unref();
        });
      child = undefined;
      endpoint = undefined;
      if (root) rmSync(root, { recursive: true, force: true });
      root = undefined;
    },
  };
}
export const directTarget = makeDirectTarget();
