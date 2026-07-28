import { runConformance } from './suite.js';
import { externalTarget } from '../targets/external.js';
import { mockTarget } from '../targets/mock.js';
import { macosTarget } from '../targets/macos.js';
import { iosTarget } from '../targets/ios.js';
import { androidTarget } from '../targets/android.js';

const name = process.argv[2] ?? 'mock';
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
