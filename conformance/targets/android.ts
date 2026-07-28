import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import type { Target } from '../src/suite.js';

const hostPort = 8766;
let serial = '';

export const androidTarget: Target = {
	name: 'android',
	async start() {
		serial = process.env.FLOAT_VIEW_ANDROID_EMULATOR ?? findEmulator();
		const root = fileURLToPath(new URL('../../', import.meta.url));
		execFileSync(`${root}/backends/android/gradlew`, ['-p', `${root}/backends/android`, 'assembleDebug'], {
			stdio: 'inherit'
		});
		adb('install', '-r', `${root}/backends/android/app/build/outputs/apk/debug/app-debug.apk`);
		adb('shell', 'pm', 'grant', 'fail.acheron.floatview', 'android.permission.BLUETOOTH_SCAN');
		adb('shell', 'pm', 'grant', 'fail.acheron.floatview', 'android.permission.BLUETOOTH_CONNECT');
		adb('forward', `tcp:${hostPort}`, 'tcp:8765');
		// Session export intentionally opens Android's system chooser. Close that
		// emulator-owned activity so repeated conformance runs relaunch our app.
		adb('shell', 'am', 'force-stop', 'com.android.intentresolver');
		adb(
			'shell',
			'am',
			'start',
			'-W',
			'-S',
			'-a',
			'android.intent.action.MAIN',
			'-c',
			'android.intent.category.LAUNCHER',
			'-n',
			'fail.acheron.floatview/fail.acheron.floatview.MainActivity'
		);
		return bootstrap();
	},
	async stop() {
		if (serial) {
			adb('shell', 'am', 'force-stop', 'fail.acheron.floatview');
			adb('forward', '--remove', `tcp:${hostPort}`);
		}
	}
};

function adb(...args: string[]) {
	execFileSync('adb', ['-s', serial, ...args], { stdio: 'inherit' });
}

function findEmulator(): string {
	const output = execFileSync('adb', ['devices'], { encoding: 'utf8' });
	const value = output
		.split('\n')
		.map((line) => line.split(/\s+/))
		.find(([name, state]) => name?.startsWith('emulator-') && state === 'device')?.[0];
	if (!value) throw new Error('Start an Android Emulator or set FLOAT_VIEW_ANDROID_EMULATOR');
	return value;
}

async function bootstrap() {
	const forwardedOrigin = `http://127.0.0.1:${hostPort}`;
	for (let attempt = 0; attempt < 100; attempt += 1) {
		try {
			const response = await fetch(`${forwardedOrigin}/api/bootstrap`);
			if (response.ok) {
				const value = (await response.json()) as { webSocketUrl: string; sessionToken: string };
				const url = new URL(value.webSocketUrl);
				url.hostname = '127.0.0.1';
				url.port = String(hostPort);
				url.searchParams.set('token', value.sessionToken);
				return { url: url.toString(), origin: 'http://127.0.0.1:8765' };
			}
		} catch {}
		await new Promise((resolve) => setTimeout(resolve, 100));
	}
	throw new Error('Android Emulator backend did not become ready');
}
