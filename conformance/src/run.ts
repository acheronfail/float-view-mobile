import { directTarget, makeDirectTarget } from '../targets/direct.ts';
import { InteractiveTelemetrySource } from '../fixture/telemetry.ts';
import {
  runConformance,
  runBoardExportConformance,
  runAnnouncementsConformance,
  runConnectionContextConformance,
  runAutoConnectConformance,
  runDiagnosticsConformance,
  runRideContinuityConformance,
  runRefloatBetaConformance,
} from './suite.ts';
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
const betaVersion = process.env.FLOAT_VIEW_FIXTURE_REFLOAT;
const target =
  name === 'direct'
    ? betaVersion === '1.3.1-beta2' || betaVersion === '1.3.1-beta3'
      ? makeDirectTarget({
          telemetry: new InteractiveTelemetrySource(),
          refloatVersion: betaVersion,
          onSpeech: () => {},
        })
      : directTarget
    : name === 'mock'
      ? mockTarget
      : name === 'macos'
        ? macosTarget
        : name === 'ios'
          ? iosTarget
          : name === 'android'
            ? androidTarget
            : externalTarget(name, `FLOAT_VIEW_${name.toUpperCase()}_WS_URL`);
const result = await (
  ['board-export', 'app-transfer'].includes(process.env.FLOAT_VIEW_CONFORMANCE_SCENARIOS ?? '')
    ? runBoardExportConformance
    : process.env.FLOAT_VIEW_CONFORMANCE_SCENARIOS === 'announcements'
      ? runAnnouncementsConformance
      : ['refloat-beta2', 'refloat-beta3'].includes(process.env.FLOAT_VIEW_CONFORMANCE_SCENARIOS ?? '')
        ? runRefloatBetaConformance
        : process.env.FLOAT_VIEW_CONFORMANCE_SCENARIOS === 'connection-context'
          ? runConnectionContextConformance
          : process.env.FLOAT_VIEW_CONFORMANCE_SCENARIOS === 'ride-continuity'
            ? runRideContinuityConformance
            : process.env.FLOAT_VIEW_CONFORMANCE_SCENARIOS === 'diagnostics'
              ? runDiagnosticsConformance
              : process.env.FLOAT_VIEW_CONFORMANCE_SCENARIOS === 'auto-connect'
                ? runAutoConnectConformance
                : runConformance
)(target);
console.log(`Conformance passed: ${result.target} revision=${result.revision}`);

function requiredTarget(value: string | undefined, label: string) {
  if (!value) throw new Error(`${label} is required`);
  return value;
}
