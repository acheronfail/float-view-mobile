import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { createInterface } from 'node:readline';
import { join, resolve as resolvePath } from 'node:path';
import WebSocket from 'ws';
import {
	runBoardProfileConformance,
	runSessionMutationConformance,
	runSessionReadConformance,
	runSystemConformance,
	type ConformanceClient
} from '../../../conformance/src/system.js';

const executable =
	process.env.FLOAT_VIEW_APPLE_HOST ?? '../apple/.build/arm64-apple-macosx/debug/FloatViewMac';
const logRoot = mkdtempSync(join(tmpdir(), 'float-view-apple-conformance-'));
const child = spawn(executable, ['--test-host'], {
	cwd: process.cwd(),
	env: {
		...process.env,
		FLOAT_VIEW_WEB_ROOT: resolvePath(process.cwd(), 'build'),
		FLOAT_VIEW_LOG_ROOT: logRoot,
		FLOAT_VIEW_TEST_SEED_PROFILE: '1'
	},
	stdio: ['ignore', 'pipe', 'inherit']
});

const readiness = await new Promise<{
	webSocketUrl: string;
	sessionToken: string;
	origin: string;
	savedDeviceId: string;
}>((resolve, reject) => {
	const lines = createInterface({ input: child.stdout });
	lines.once('line', (line) => {
		try {
			resolve(JSON.parse(line));
		} catch (error) {
			reject(error);
		}
	});
	child.once('exit', (code) => reject(new Error(`Apple host exited early: ${code}`)));
});

// FlyingFox reports its bound address before the accept loop is observable by
// a separate process on some macOS versions.
await new Promise((resolve) => setTimeout(resolve, 100));

let socketToClose: WebSocket | undefined;
const sharedClient = new Promise<ConformanceClient>((resolve, reject) => {
	const url = new URL(readiness.webSocketUrl);
	url.searchParams.set('token', readiness.sessionToken);
	const socket = new WebSocket(url, { origin: readiness.origin });
	socketToClose = socket;
	socket.once('open', () => {
		let id = 0;
		const pending = new Map<
			number,
			{ resolve(value: unknown): void; reject(error: unknown): void }
		>();
		socket.on('message', (data) => {
			const message = JSON.parse(data.toString());
			const request = pending.get(message.id);
			if (!request) return;
			pending.delete(message.id);
			if (message.error) request.reject(message.error);
			else request.resolve(message.result);
		});
		resolve({
			request(method, params) {
				const requestId = ++id;
				return new Promise((resolveRequest, rejectRequest) => {
					pending.set(requestId, { resolve: resolveRequest, reject: rejectRequest });
					socket.send(JSON.stringify({ jsonrpc: '2.0', id: requestId, method, params }));
				});
			},
			close() {
				// The four system scenarios intentionally share one connection so
				// they also prove ordered state progression.
			}
		});
	});
	socket.once('error', reject);
});

const results = await runSystemConformance(() => sharedClient, { includeProductScenarios: false });
const client = await sharedClient;
results.push(await runBoardProfileConformance(client, readiness.savedDeviceId));
results.push(await runSessionReadConformance(client));
results.push(await runSessionMutationConformance(client));

for (const result of results)
	console.log(`PASS ${result.name} (${result.durationMs.toFixed(1)}ms)`);
socketToClose?.terminate();
child.kill('SIGTERM');
rmSync(logRoot, { recursive: true, force: true });
console.log(`Apple loopback host passed ${results.length} system conformance scenarios.`);
