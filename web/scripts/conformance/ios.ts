import { execFile, spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { promisify } from 'node:util';
import WebSocket from 'ws';
import {
	runBoardProfileConformance,
	runSessionMutationConformance,
	runSessionReadConformance,
	runSystemConformance,
	type ConformanceClient
} from '../../../conformance/src/system.js';

const execFileAsync = promisify(execFile);
const repositoryRoot = new URL('../../..', import.meta.url).pathname;
const derivedData = '/tmp/float-view-ios-conformance';
const bundleId = 'fail.acheron.float-view.hardware-host';
const simulator = await resolveSimulator();

await command('xcodebuild', [
	'-project',
	`${repositoryRoot}/ios/float-view.xcodeproj`,
	'-scheme',
	'float-view-hardware',
	'-configuration',
	'Debug',
	'-destination',
	`platform=iOS Simulator,id=${simulator}`,
	'-derivedDataPath',
	derivedData,
	'CODE_SIGNING_ALLOWED=NO',
	'build-for-testing'
]);
const appPath = `${derivedData}/Build/Products/Debug-iphonesimulator/float-view-hardware-host.app`;
await command('xcrun', ['simctl', 'install', simulator, appPath]);

const child = spawn(
	'xcrun',
	[
		'simctl',
		'launch',
		'--console',
		'--terminate-running-process',
		simulator,
		bundleId,
		'--conformance-host'
	],
	{ cwd: repositoryRoot, stdio: ['ignore', 'pipe', 'pipe'] }
);
child.stderr.on('data', (value) => process.stderr.write(value));
const readiness = await new Promise<{
	webSocketUrl: string;
	sessionToken: string;
	origin: string;
	savedDeviceId: string;
}>((resolve, reject) => {
	const lines = createInterface({ input: child.stdout });
	lines.on('line', (line) => {
		try {
			const value = JSON.parse(line);
			if (value.webSocketUrl) resolve(value);
		} catch {
			// Simulator launch diagnostics may precede application stdout.
		}
	});
	child.once('exit', (code) => reject(new Error(`iOS conformance host exited early: ${code}`)));
});

let socketToClose: WebSocket | undefined;
try {
	const url = new URL(readiness.webSocketUrl);
	url.searchParams.set('token', readiness.sessionToken);
	const sharedClient = new Promise<ConformanceClient>((resolve, reject) => {
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
					// Native seeded scenarios intentionally share one connection.
				}
			});
		});
		socket.once('error', reject);
	});
	const results = await runSystemConformance(() => sharedClient, {
		includeProductScenarios: false
	});
	const client = await sharedClient;
	results.push(await runBoardProfileConformance(client, readiness.savedDeviceId));
	results.push(await runSessionReadConformance(client));
	results.push(await runSessionMutationConformance(client));
	for (const result of results) {
		console.log(`PASS ${result.name} (${result.durationMs.toFixed(1)}ms)`);
	}
	console.log(`iOS loopback host passed ${results.length} system conformance scenarios.`);
} finally {
	socketToClose?.terminate();
	await command('xcrun', ['simctl', 'terminate', simulator, bundleId]).catch(() => undefined);
}

async function resolveSimulator(): Promise<string> {
	const devices = JSON.parse(
		await command('xcrun', ['simctl', 'list', 'devices', 'available', '-j'])
	).devices as Record<string, Array<{ udid: string; name: string; state: string }>>;
	const available = Object.values(devices).flat();
	const booted = available.find((device) => device.state === 'Booted');
	if (booted) return booted.udid;
	const selected = available.find((device) => device.name === 'iPhone 16') ?? available[0];
	if (!selected) throw new Error('No available iOS simulator runtime is installed');
	await command('xcrun', ['simctl', 'boot', selected.udid]);
	await command('xcrun', ['simctl', 'bootstatus', selected.udid, '-b']);
	return selected.udid;
}

async function command(executable: string, args: string[]): Promise<string> {
	const result = await execFileAsync(executable, args, {
		cwd: repositoryRoot,
		maxBuffer: 20 * 1024 * 1024
	});
	return result.stdout;
}
