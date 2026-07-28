import { execFile, spawn } from 'node:child_process';
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
const applicationId = 'fail.acheron.floatview';
const serial = process.env.ANDROID_SERIAL ?? (await resolveAndroidSerial());

await command(
	'./gradlew',
	['--no-daemon', ':app:installDebug', ':app:installDebugAndroidTest'],
	`${repositoryRoot}/android`
);
await command('adb', ['-s', serial, 'shell', 'pm', 'clear', applicationId]);

const instrumentation = spawn(
	'adb',
	[
		'-s',
		serial,
		'shell',
		'am',
		'instrument',
		'-w',
		'-r',
		'-e',
		'class',
		`${applicationId}.AndroidConformanceHostTest`,
		`${applicationId}.test/androidx.test.runner.AndroidJUnitRunner`
	],
	{ cwd: repositoryRoot, stdio: ['ignore', 'pipe', 'pipe'] }
);
let instrumentationOutput = '';
instrumentation.stdout.on('data', (value) => (instrumentationOutput += value.toString()));
instrumentation.stderr.on('data', (value) => (instrumentationOutput += value.toString()));
const instrumentationExit = new Promise<number | null>((resolve) =>
	instrumentation.once('exit', resolve)
);

let forwardPort: string | undefined;
let socketToClose: WebSocket | undefined;
let conformanceError: unknown;
try {
	const readiness = await waitForReadiness(serial);
	forwardPort = (
		await command('adb', ['-s', serial, 'forward', 'tcp:0', `tcp:${readiness.port}`])
	).trim();
	const bootstrap = (await fetch(`http://127.0.0.1:${forwardPort}/api/bootstrap`).then((response) =>
		response.json()
	)) as {
		webSocketUrl: string;
		sessionToken: string;
	};
	const socketUrl = new URL(bootstrap.webSocketUrl);
	socketUrl.hostname = '127.0.0.1';
	socketUrl.port = forwardPort;
	socketUrl.searchParams.set('token', bootstrap.sessionToken);

	const sharedClient = new Promise<ConformanceClient>((resolve, reject) => {
		const socket = new WebSocket(socketUrl, { origin: readiness.origin });
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
	console.log(`Android loopback host passed ${results.length} system conformance scenarios.`);
} catch (error) {
	conformanceError = error;
} finally {
	socketToClose?.terminate();
	await command('adb', [
		'-s',
		serial,
		'shell',
		'am',
		'broadcast',
		'-a',
		'fail.acheron.floatview.R5_CONFORMANCE_STOP',
		'-p',
		applicationId
	]).catch(() => undefined);
	if (forwardPort) {
		await command('adb', ['-s', serial, 'forward', '--remove', `tcp:${forwardPort}`]).catch(
			() => undefined
		);
	}
}

const exitCode = await instrumentationExit;
if (conformanceError) {
	throw conformanceError;
}
if (exitCode !== 0 || !instrumentationOutput.includes('OK (1 test)')) {
	throw new Error(
		`Android conformance instrumentation failed (${exitCode})\n${instrumentationOutput}`
	);
}

async function resolveAndroidSerial(): Promise<string> {
	const output = await command('adb', ['devices']);
	const devices = output
		.split(/\r?\n/)
		.map((line) => line.trim().split(/\s+/))
		.filter((fields) => fields.length >= 2 && fields[1] === 'device')
		.map((fields) => fields[0]!);
	if (devices.length !== 1) {
		throw new Error(`Expected exactly one authorized Android destination, found ${devices.length}`);
	}
	return devices[0]!;
}

async function waitForReadiness(
	deviceSerial: string
): Promise<{ port: number; origin: string; savedDeviceId: string }> {
	const deadline = performance.now() + 30_000;
	while (performance.now() < deadline) {
		try {
			const output = await command('adb', [
				'-s',
				deviceSerial,
				'exec-out',
				'run-as',
				applicationId,
				'cat',
				'cache/r5-android-conformance.json'
			]);
			return JSON.parse(output);
		} catch {
			await new Promise((resolve) => setTimeout(resolve, 100));
		}
	}
	throw new Error('Android conformance host did not publish readiness within 30 seconds');
}

async function command(executable: string, args: string[], cwd = repositoryRoot): Promise<string> {
	const result = await execFileAsync(executable, args, { cwd, maxBuffer: 10 * 1024 * 1024 });
	return result.stdout;
}
