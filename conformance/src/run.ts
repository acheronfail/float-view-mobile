import { runConformance } from './suite.ts';
import { externalTarget } from '../targets/external.ts';
import { mockTarget } from '../targets/mock.ts';
import { macosTarget } from '../targets/macos.ts';
import { iosTarget } from '../targets/ios.ts';
import { androidTarget } from '../targets/android.ts';
import { androidPhysicalTarget } from '../targets/android.ts';
import { iosPhysicalTarget } from '../targets/ios.ts';
import { macosPhysicalTarget } from '../targets/macos.ts';
import { runPhysicalConformance } from './physical.ts';

const name = process.argv[2] ?? 'mock';
if (name === 'physical') {
  const platform = process.argv[3];
  const fixture = process.argv[4];
  const targetId = process.argv[5];
  if (!fixture || !platform || !['android', 'ios', 'macos'].includes(platform)) {
    throw new Error('Usage: run.ts physical android|ios|macos <fixture-port> [device-id]');
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
