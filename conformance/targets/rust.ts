import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import type { Target } from '../src/suite.ts';

const core = fileURLToPath(new URL('../../backends/core', import.meta.url));
const endpointPrefix = 'FLOAT_VIEW_RUST_WS_URL=';

let child: ChildProcessWithoutNullStreams | undefined;
let currentEndpoint: { url: string; origin: string } | undefined;

export const rustTarget: Target = {
  name: 'rust',
  async start() {
    if (child) throw new Error('Standalone Rust backend is already running');
    child = spawn('cargo', ['run', '-p', 'test-standalone', '--features', 'software-host', '--offline'], {
      cwd: core,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    const process = child;
    let diagnostics = '';
    process.stderr.setEncoding('utf8');
    process.stderr.on('data', (chunk: string) => {
      diagnostics = `${diagnostics}${chunk}`.slice(-8_192);
    });
    const url = await new Promise<string>((resolve, reject) => {
      const lines = createInterface({ input: process.stdout });
      const timeout = setTimeout(() => {
        lines.close();
        reject(new Error(`Timed out starting standalone Rust backend\n${diagnostics}`));
      }, 30_000);
      const failed = (code: number | null) => {
        clearTimeout(timeout);
        lines.close();
        reject(new Error(`Standalone Rust backend exited (${code ?? 'signal'})\n${diagnostics}`));
      };
      process.once('exit', failed);
      lines.on('line', (line) => {
        if (!line.startsWith(endpointPrefix)) return;
        clearTimeout(timeout);
        process.off('exit', failed);
        lines.close();
        resolve(line.slice(endpointPrefix.length));
      });
    });
    const parsed = new URL(url);
    currentEndpoint = { url, origin: `http://${parsed.host}` };
    return currentEndpoint;
  },
  async endpoint() {
    if (!currentEndpoint) throw new Error('Standalone Rust backend is not running');
    return currentEndpoint;
  },
  async stop() {
    const process = child;
    child = undefined;
    currentEndpoint = undefined;
    if (!process || process.exitCode !== null) return;
    await new Promise<void>((resolve) => {
      const timeout = setTimeout(() => {
        process.kill('SIGTERM');
        resolve();
      }, 5_000);
      process.once('exit', () => {
        clearTimeout(timeout);
        resolve();
      });
      process.stdin.end('\n');
    });
  },
};
