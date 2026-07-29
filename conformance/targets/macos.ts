import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import type { PhysicalTarget, Target } from '../src/suite.ts';

export const macosTarget: Target = makeMacosTarget(false);
export const macosPhysicalTarget: PhysicalTarget = {
	...makeMacosTarget(true),
	name: 'macos-physical',
	platform: 'macos',
	productionBluetooth: true
};

function makeMacosTarget(physical: boolean): Target {
	let child: ChildProcess | undefined;
	let logRoot: string | undefined;
	let currentEndpoint: { url: string; origin: string } | undefined;
	return {
		name: physical ? 'macos-physical' : 'macos',
		async start() {
			const root = fileURLToPath(new URL('../../', import.meta.url));
			const executable =
				process.env.FLOAT_VIEW_MACOS_EXECUTABLE ??
				buildMacosExecutable(root);
			logRoot = mkdtempSync(join(tmpdir(), `float-view-macos-${physical ? 'physical' : 'software'}-`));
			const env: NodeJS.ProcessEnv = {
				...process.env,
				FLOAT_VIEW_FRONTEND_HTML: `${root}/frontend/build/index.html`,
				FLOAT_VIEW_SQL_ROOT: `${root}/backends/sql`,
				FLOAT_VIEW_LOG_ROOT: logRoot
			};
			if (physical) {
				delete env.FLOAT_VIEW_SOFTWARE_BACKEND;
				delete env.FLOAT_VIEW_TEST_SEED_PROFILE;
			} else {
				env.FLOAT_VIEW_SOFTWARE_BACKEND = '1';
				env.FLOAT_VIEW_TEST_SEED_PROFILE = '1';
			}
			child = spawn(executable, macosLaunchArguments(), {
				cwd: root,
				env,
				stdio: ['ignore', 'pipe', 'inherit']
			});
			const readiness = await firstLine(child.stdout!);
			const value = JSON.parse(readiness) as {
				webSocketUrl: string;
				sessionToken: string;
				origin: string;
			};
			currentEndpoint = {
				url: `${value.webSocketUrl}?token=${encodeURIComponent(value.sessionToken)}`,
				origin: value.origin
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
			if (logRoot) rmSync(logRoot, { recursive: true, force: true });
			logRoot = undefined;
		}
	};
}

export function macosLaunchArguments() {
	return ['--test-host'];
}

function buildMacosExecutable(root: string) {
	const cwd = `${root}/backends/apple`;
	const scratch = '/tmp/float-view-swift-build';
	execFileSync(
		'swift',
		['build', '--scratch-path', scratch, '--product', 'FloatViewMac'],
		{ cwd, stdio: 'inherit' }
	);
	const binaryRoot = execFileSync(
		'swift',
		['build', '--scratch-path', scratch, '--show-bin-path'],
		{ cwd, encoding: 'utf8' }
	).trim();
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
