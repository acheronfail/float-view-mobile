import { spawn, type ChildProcess } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import type { Target } from '../src/suite.js';

let process: ChildProcess | undefined;

export const macosTarget: Target = {
	name: 'macos',
	async start() {
		const root = fileURLToPath(new URL('../../', import.meta.url));
		const executable =
			globalThis.process.env.FLOAT_VIEW_MACOS_EXECUTABLE ??
			'/tmp/float-view-swift-build/arm64-apple-macosx/debug/FloatViewMac';
		process = spawn(executable, ['--test-host'], {
			cwd: root,
			env: {
				...globalThis.process.env,
				FLOAT_VIEW_SOFTWARE_BACKEND: '1',
				FLOAT_VIEW_TEST_SEED_PROFILE: '1',
				FLOAT_VIEW_FRONTEND_HTML: `${root}/frontend/build/index.html`,
				FLOAT_VIEW_SQL_ROOT: `${root}/backends/sql`,
				FLOAT_VIEW_LOG_ROOT: `/tmp/float-view-macos-conformance-${globalThis.process.pid}`
			},
			stdio: ['ignore', 'pipe', 'inherit']
		});
		const readiness = await firstLine(process.stdout!);
		const value = JSON.parse(readiness) as {
			webSocketUrl: string;
			sessionToken: string;
			origin: string;
		};
		return {
			url: `${value.webSocketUrl}?token=${encodeURIComponent(value.sessionToken)}`,
			origin: value.origin
		};
	},
	async stop() {
		process?.kill('SIGTERM');
		process = undefined;
	}
};

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
