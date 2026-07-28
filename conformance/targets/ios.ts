import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import type { Target } from '../src/suite.js';

const port = 8765;
let device = '';

export const iosTarget: Target = {
	name: 'ios',
	async start() {
		device = process.env.FLOAT_VIEW_IOS_SIMULATOR ?? findSimulator();
		const root = fileURLToPath(new URL('../../', import.meta.url));
		const project = `${root}/backends/apple/ios/float-view.xcodeproj`;
		const derived = '/tmp/float-view-ios-conformance';
		try {
			execFileSync('xcrun', ['simctl', 'boot', device], { stdio: 'ignore' });
		} catch {
			// bootstatus below distinguishes an already-running simulator from a real failure.
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
		execFileSync('xcrun', [
			'simctl',
			'install',
			device,
			`${derived}/Build/Products/Debug-iphonesimulator/float-view.app`
		]);
		execFileSync(
			'xcrun',
			['simctl', 'launch', '--terminate-running-process', device, 'fail.acheron.float-view'],
			{ env: { ...process.env, SIMCTL_CHILD_FLOAT_VIEW_CONFORMANCE_PORT: String(port) } }
		);
		return bootstrap(port);
	},
	async stop() {
		execFileSync('xcrun', ['simctl', 'terminate', device, 'fail.acheron.float-view'], {
			stdio: 'ignore'
		});
	}
};

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

async function bootstrap(port: number) {
	const origin = `http://127.0.0.1:${port}`;
	for (let attempt = 0; attempt < 100; attempt += 1) {
		try {
			const response = await fetch(`${origin}/api/bootstrap`);
			if (response.ok) {
				const value = (await response.json()) as { webSocketUrl: string; sessionToken: string };
				return {
					url: `${value.webSocketUrl}?token=${encodeURIComponent(value.sessionToken)}`,
					origin
				};
			}
		} catch {}
		await new Promise((resolve) => setTimeout(resolve, 100));
	}
	throw new Error('iOS Simulator backend did not become ready');
}
