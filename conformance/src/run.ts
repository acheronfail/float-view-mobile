import { runConformance } from './suite.js';
import { externalTarget } from '../targets/external.js';
import { mockTarget } from '../targets/mock.js';
import { macosTarget } from '../targets/macos.js';
import { iosTarget } from '../targets/ios.js';
import { androidTarget } from '../targets/android.js';
import { androidPhysicalTarget } from '../targets/android.js';
import { iosPhysicalTarget } from '../targets/ios.js';
import { macosPhysicalTarget } from '../targets/macos.js';
import { runPhysicalConformance } from './physical.js';

const name = process.argv[2] ?? 'mock';
if (name === 'physical') {
	const platform = process.argv[3];
	const fixture = process.argv[4];
	const targetId = process.argv[5];
	if (!fixture || !platform || !['android', 'ios', 'macos'].includes(platform)) {
		throw new Error(
			'Usage: run.ts physical android|ios|macos <fixture-port> [device-id]'
		);
	}
	const target =
		platform === 'android'
			? androidPhysicalTarget(requiredTarget(targetId, 'Android ADB serial'))
			: platform === 'ios'
				? iosPhysicalTarget(requiredTarget(targetId, 'iOS device ID'))
				: macosPhysicalTarget;
	const result = await runPhysicalConformance(target, fixture);
	console.log(
		`Physical conformance passed: ${result.target} scenarios=${result.scenarios.map((item) => item.scenario).join(',')}`
	);
	process.exit(0);
}
const target =
	name === 'mock'
		? mockTarget
		: name === 'macos'
			? macosTarget
			: name === 'ios'
				? iosTarget
				: name === 'android'
					? androidTarget
		: externalTarget(name, `FLOAT_VIEW_${name.toUpperCase()}_WS_URL`);
const result = await runConformance(target);
console.log(`Conformance passed: ${result.target} revision=${result.revision}`);

function requiredTarget(value: string | undefined, label: string) {
	if (!value) throw new Error(`${label} is required`);
	return value;
}
