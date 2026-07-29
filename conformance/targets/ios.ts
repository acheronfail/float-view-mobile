import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import type { PhysicalTarget, Target } from '../src/suite.ts';

const devicePort = 8765;
const hostPort = 8766;
const bundleId = 'fail.acheron.float-view';
const lifecycleRunnerBundleId = 'fail.acheron.float-view.lifecycle-tests.xctrunner';
const MAX_COMMAND_OUTPUT = 64 * 1_024;
const BUILD_TIMEOUT_MS = 5 * 60_000;
const DEVICE_TIMEOUT_MS = 60_000;
const CLEANUP_TIMEOUT_MS = 15_000;
const LIFECYCLE_TIMEOUT_MS = 2 * 60_000;
const DESTINATION_TIMEOUT_SECONDS = 30;
const DEVICE_PREPARATION_ATTEMPTS = 3;

export const iosTarget: Target = makeSimulatorTarget();

export function iosPhysicalTarget(deviceId: string): PhysicalTarget {
	if (!deviceId) throw new Error('iOS device ID is required');
	let appPid: number | undefined;
	let proxy: ChildProcess | undefined;
	let proxyFailure: Error | undefined;
	let proxyDiagnostics = '';
	let stoppingProxy = false;
	let prepared = false;
	let scratch: string | undefined;
	let launchSequence = 0;
	const root = fileURLToPath(new URL('../../', import.meta.url));
	const project = `${root}/backends/apple/ios/float-view.xcodeproj`;
	const derived = '/tmp/float-view-ios-physical-conformance';
	const destination = `platform=iOS,id=${deviceId}`;
	const lifecycle = (method: 'testBackground' | 'testForeground') =>
		runBuffered('xcodebuild', [
				'-project',
				project,
				'-scheme',
				'float-view',
				'-configuration',
				'Debug',
				'-destination',
				destination,
				'-destination-timeout',
				String(DESTINATION_TIMEOUT_SECONDS),
				'-derivedDataPath',
				derived,
				'-allowProvisioningUpdates',
				`-only-testing:float-view-lifecycle-tests/FloatViewLifecycleTests/${method}`,
				'test-without-building'
			], `iOS lifecycle ${method}`, LIFECYCLE_TIMEOUT_MS);
	const prepareSuite = async (report: (message: string) => void = () => undefined) => {
		if (prepared) return;
		report('iOS: building application...');
		await retryWhileDevicePrepares(
			() =>
				runBuffered(
					'xcodebuild',
					iosPhysicalBuildArguments(project, destination, derived),
					'iOS application build',
					BUILD_TIMEOUT_MS
				),
			report
		);
		report('iOS: application built');
		report('iOS: installing application...');
		await runBuffered(
			'xcrun',
			[
				'devicectl',
				'device',
				'install',
				'app',
				'--device',
				deviceId,
				`${derived}/Build/Products/Debug-iphoneos/float-view.app`
			],
			'iOS application install',
			DEVICE_TIMEOUT_MS
		);
		report('iOS: application installed');
		report('iOS: starting loopback proxy...');
		scratch = mkdtempSync(join(tmpdir(), 'float-view-ios-physical-'));
		proxy = spawn('iproxy', ['--udid', deviceId, `${hostPort}:${devicePort}`], {
			stdio: ['ignore', 'ignore', 'pipe']
		});
		proxy.stderr?.on('data', (chunk: Buffer) => {
			if (proxyDiagnostics.length < 4_096) {
				proxyDiagnostics += chunk.toString().slice(0, 4_096 - proxyDiagnostics.length);
			}
		});
		proxy.once('error', (error) => {
			proxyFailure = new Error(`iproxy failed: ${error.message}`);
		});
		proxy.once('exit', (code, signal) => {
			if (!stoppingProxy) {
				proxyFailure = new Error(
					`iproxy exited${signal ? ` with signal ${signal}` : ` with exit code ${code ?? 'unknown'}`}`
				);
			}
		});
		prepared = true;
		report('iOS: loopback proxy started');
	};
	const checkProxy = () => {
		if (!proxyFailure) return;
		const diagnostics = proxyDiagnostics.trim();
		throw new Error(
			`${proxyFailure.message}${diagnostics ? `\niproxy diagnostics:\n${diagnostics}` : ''}`
		);
	};
	const terminateApp = async () => {
		if (appPid === undefined) return;
		const pid = appPid;
		appPid = undefined;
		await runBuffered(
			'xcrun',
			iosPhysicalTerminateArguments(deviceId, pid),
			'iOS application termination',
			CLEANUP_TIMEOUT_MS
		);
	};
	return {
		name: 'ios-physical',
		platform: 'ios',
		productionBluetooth: true,
		prepareSuite,
		async finishSuite(report) {
			let cleanupFailure: Error | undefined;
			if (appPid !== undefined) report('iOS: terminating application...');
			await terminateApp().catch((error) => {
				cleanupFailure = error instanceof Error ? error : new Error(String(error));
			});
			report('iOS: stopping loopback proxy...');
			stoppingProxy = true;
			await stopChild(proxy);
			proxy = undefined;
			proxyFailure = undefined;
			proxyDiagnostics = '';
			stoppingProxy = false;
			prepared = false;
			if (scratch) rmSync(scratch, { recursive: true, force: true });
			scratch = undefined;
			report('iOS: removing lifecycle runner...');
			await runBuffered(
				'xcrun',
				[
					'devicectl',
					'device',
					'uninstall',
					'app',
					'--device',
					deviceId,
					lifecycleRunnerBundleId
				],
				'iOS lifecycle runner removal',
				CLEANUP_TIMEOUT_MS
			).catch(() => undefined);
			report('iOS: cleanup complete');
			if (cleanupFailure) throw cleanupFailure;
		},
		async start() {
			await prepareSuite();
			checkProxy();
			if (!scratch) throw new Error('iOS physical target has no temporary launch directory');
			const launchResult = join(scratch, `launch-${++launchSequence}.json`);
			await runBuffered(
				'xcrun',
				iosPhysicalLaunchArguments(deviceId, launchResult),
				'iOS application launch',
				DEVICE_TIMEOUT_MS
			);
			appPid = iosPhysicalProcessIdentifier(
				JSON.parse(readFileSync(launchResult, 'utf8')) as unknown
			);
			return bootstrap(hostPort, devicePort, 'iOS device', checkProxy);
		},
		endpoint: () => bootstrap(hostPort, devicePort, 'iOS device', checkProxy),
		background: () => lifecycle('testBackground'),
		foreground: () => lifecycle('testForeground'),
		async stop() {
			await terminateApp();
		}
	};
}

export function iosPhysicalBuildArguments(project: string, destination: string, derived: string) {
	return [
		'-project',
		project,
		'-scheme',
		'float-view',
		'-configuration',
		'Debug',
		'-destination',
		destination,
		'-destination-timeout',
		String(DESTINATION_TIMEOUT_SECONDS),
		'-derivedDataPath',
		derived,
		'-allowProvisioningUpdates',
		'build-for-testing'
	];
}

export function iosPhysicalLaunchArguments(deviceId: string, jsonOutput: string) {
	return [
		'devicectl',
		'device',
		'process',
		'launch',
		'--device',
		deviceId,
		'--environment-variables',
		JSON.stringify({
			FLOAT_VIEW_CONFORMANCE_PORT: String(devicePort),
			FLOAT_VIEW_CONFORMANCE_RESET: '1'
		}),
		'--terminate-existing',
		'--quiet',
		'--json-output',
		jsonOutput,
		bundleId
	];
}

export function iosPhysicalTerminateArguments(deviceId: string, pid: number) {
	if (!Number.isInteger(pid) || pid <= 0) throw new Error('iOS application PID is invalid');
	return [
		'devicectl',
		'device',
		'process',
		'terminate',
		'--device',
		deviceId,
		'--pid',
		String(pid),
		'--quiet'
	];
}

export function iosPhysicalProcessIdentifier(value: unknown) {
	const pid = (value as { result?: { process?: { processIdentifier?: unknown } } })?.result?.process
		?.processIdentifier;
	if (!Number.isInteger(pid) || (pid as number) <= 0) {
		throw new Error('devicectl launch returned no valid application PID');
	}
	return pid as number;
}

export function iosDeviceIsPreparingFailure(error: unknown) {
	const message = error instanceof Error ? error.message : String(error);
	return message.includes('Device is busy (Preparing');
}

function makeSimulatorTarget(): Target {
	let device = '';
	return {
		name: 'ios',
		async start() {
			device = process.env.FLOAT_VIEW_IOS_SIMULATOR ?? findSimulator();
			const root = fileURLToPath(new URL('../../', import.meta.url));
			const project = `${root}/backends/apple/ios/float-view.xcodeproj`;
			const derived = '/tmp/float-view-ios-conformance';
			try {
				execFileSync('xcrun', ['simctl', 'boot', device], { stdio: 'ignore' });
			} catch {
				// bootstatus distinguishes an already-running simulator from a real failure.
			}
			execFileSync('xcrun', ['simctl', 'bootstatus', device, '-b'], { stdio: 'inherit' });
			execFileSync(
				'xcodebuild',
				[
					'-project',
					project,
					'-scheme',
					'float-view',
					'-sdk',
					'iphonesimulator',
					'-destination',
					`id=${device}`,
					'-derivedDataPath',
					derived,
					'CODE_SIGNING_ALLOWED=NO',
					'build'
				],
				{ stdio: 'inherit' }
			);
			try {
				execFileSync('xcrun', ['simctl', 'uninstall', device, bundleId], { stdio: 'ignore' });
			} catch {
				// A clean simulator may not have the application installed.
			}
			execFileSync('xcrun', [
				'simctl',
				'install',
				device,
				`${derived}/Build/Products/Debug-iphonesimulator/float-view.app`
			]);
			execFileSync(
				'xcrun',
				['simctl', 'launch', '--terminate-running-process', device, bundleId],
				{
					env: {
						...process.env,
						SIMCTL_CHILD_FLOAT_VIEW_CONFORMANCE_PORT: String(devicePort)
					}
				}
			);
			return bootstrap(devicePort, devicePort, 'iOS Simulator');
		},
		endpoint: () => bootstrap(devicePort, devicePort, 'iOS Simulator'),
		async stop() {
			if (!device) return;
			execFileSync('xcrun', ['simctl', 'terminate', device, bundleId], { stdio: 'ignore' });
			device = '';
		}
	};
}

function findSimulator(): string {
	const value = JSON.parse(
		execFileSync('xcrun', ['simctl', 'list', 'devices', 'available', '--json'], {
			encoding: 'utf8'
		})
	) as { devices: Record<string, { udid: string; state: string }[]> };
	const devices = Object.values(value.devices).flat();
	return (
		devices.find((candidate) => candidate.state === 'Booted')?.udid ??
		devices[0]?.udid ??
		(() => {
			throw new Error('No iOS Simulator is installed');
		})()
	);
}

async function bootstrap(
	host: number,
	expected: number,
	label: string,
	checkTransport?: () => void
) {
	const forwardedOrigin = `http://127.0.0.1:${host}`;
	for (let attempt = 0; attempt < 150; attempt += 1) {
		checkTransport?.();
		try {
			const response = await fetch(`${forwardedOrigin}/api/bootstrap`);
			if (response.ok) {
				const value = (await response.json()) as {
					webSocketUrl: string;
					sessionToken: string;
				};
				const url = new URL(value.webSocketUrl);
				url.hostname = '127.0.0.1';
				url.port = String(host);
				url.searchParams.set('token', value.sessionToken);
				return { url: url.toString(), origin: `http://127.0.0.1:${expected}` };
			}
		} catch {}
		await new Promise((resolve) => setTimeout(resolve, 100));
	}
	checkTransport?.();
	throw new Error(`${label} backend did not become ready`);
}

async function stopChild(child: ChildProcess | undefined) {
	if (!child || child.exitCode !== null || child.signalCode !== null) return;
	await new Promise<void>((resolve) => {
		const timer = setTimeout(() => child.kill('SIGKILL'), 2_000);
		child.once('close', () => {
			clearTimeout(timer);
			resolve();
		});
		child.kill('SIGTERM');
	});
}

function runBuffered(
	command: string,
	arguments_: string[],
	label: string,
	timeoutMs: number
) {
	return new Promise<void>((resolve, reject) => {
		const child = spawn(command, arguments_, { stdio: ['ignore', 'pipe', 'pipe'] });
		let output = '';
		let timedOut = false;
		let forceTimer: NodeJS.Timeout | undefined;
		const timer = setTimeout(() => {
			timedOut = true;
			child.kill('SIGTERM');
			forceTimer = setTimeout(() => child.kill('SIGKILL'), 2_000);
		}, timeoutMs);
		const append = (chunk: Buffer) => {
			output = `${output}${chunk.toString()}`.slice(-MAX_COMMAND_OUTPUT);
		};
		child.stdout?.on('data', append);
		child.stderr?.on('data', append);
		child.once('error', (error) => {
			clearTimeout(timer);
			if (forceTimer) clearTimeout(forceTimer);
			reject(new Error(`${label} failed: ${error.message}`));
		});
		child.once('close', (code, signal) => {
			clearTimeout(timer);
			if (forceTimer) clearTimeout(forceTimer);
			if (code === 0) {
				resolve();
				return;
			}
			const diagnostics = output.trim();
			reject(
				new Error(
					`${label} ${timedOut ? `timed out after ${Math.round(timeoutMs / 1_000)}s` : `failed${signal ? ` with signal ${signal}` : ` with exit code ${code ?? 'unknown'}`}`}${diagnostics ? `\n${diagnostics}` : ''}`
				)
			);
		});
	});
}

async function retryWhileDevicePrepares(
	action: () => Promise<void>,
	report: (message: string) => void
) {
	for (let attempt = 1; attempt <= DEVICE_PREPARATION_ATTEMPTS; attempt += 1) {
		try {
			await action();
			return;
		} catch (error) {
			if (!iosDeviceIsPreparingFailure(error) || attempt === DEVICE_PREPARATION_ATTEMPTS) {
				throw error;
			}
			report(
				`iOS: device is still preparing; retrying build (${attempt + 1}/${DEVICE_PREPARATION_ATTEMPTS})...`
			);
			await new Promise((resolve) => setTimeout(resolve, 2_000));
		}
	}
}
