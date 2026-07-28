import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { validateProfile } from './hardware-smoke.mjs';

const root = resolve(import.meta.dirname, '..');
const forbiddenEvidenceKeys = new Set([
  'address',
  'bleAddress',
  'deviceId',
  'peripheralUuid',
  'rawPackets',
  'sessionId',
  'sessionToken',
  'telemetry'
]);

function invariant(condition, message) {
  if (!condition) throw new Error(message);
}

async function loadJson(path) {
  return JSON.parse(await readFile(path, 'utf8'));
}

function assertRedacted(value, path = 'evidence') {
  if (Array.isArray(value)) {
    value.forEach((item, index) => assertRedacted(item, `${path}[${index}]`));
    return;
  }
  if (!value || typeof value !== 'object') return;
  for (const [key, child] of Object.entries(value)) {
    invariant(!forbiddenEvidenceKeys.has(key), `sensitive field is forbidden at ${path}.${key}`);
    assertRedacted(child, `${path}.${key}`);
  }
}

export function createFailureEvidenceTemplate(spec, profile, platform) {
  invariant(spec.schemaVersion === 1, 'unsupported failure-scenario schema');
  invariant(spec.supportedPlatforms.includes(platform), `unsupported platform: ${platform}`);
  return {
    schemaVersion: 1,
    scenarioSetId: spec.id,
    profileId: profile.id,
    platform,
    recordedAt: null,
    hostQualification: null,
    identityRedacted: true,
    scenarios: Object.fromEntries(
      spec.scenarios.map((scenario) => [
        scenario.id,
        {
          passed: null,
          observations: Object.fromEntries(
            Object.keys(scenario.expected).map((key) => [key, null])
          ),
          notes: null
        }
      ])
    )
  };
}

export function validateFailureEvidence(spec, profile, evidence, platform) {
  invariant(spec.schemaVersion === 1, 'unsupported failure-scenario schema');
  invariant(evidence.schemaVersion === 1, 'unsupported failure-evidence schema');
  invariant(evidence.scenarioSetId === spec.id, 'failure-scenario set mismatch');
  invariant(evidence.profileId === profile.id, 'failure-evidence profile mismatch');
  invariant(evidence.platform === platform, 'failure-evidence platform mismatch');
  invariant(evidence.identityRedacted === true, 'failure evidence must be redacted');
  invariant(
    typeof evidence.recordedAt === 'string' && !Number.isNaN(Date.parse(evidence.recordedAt)),
    'recordedAt must be an ISO timestamp'
  );
  invariant(
    typeof evidence.hostQualification === 'string' && evidence.hostQualification.length > 0,
    'hostQualification is required'
  );
  assertRedacted(evidence);

  for (const scenario of spec.scenarios) {
    const result = evidence.scenarios?.[scenario.id];
    invariant(result, `missing scenario: ${scenario.id}`);
    invariant(result.passed === true, `scenario did not pass: ${scenario.id}`);
    for (const [key, expected] of Object.entries(scenario.expected)) {
      invariant(
        Object.hasOwn(result.observations ?? {}, key),
        `missing ${scenario.id} observation: ${key}`
      );
      invariant(
        result.observations[key] === expected,
        `${scenario.id}.${key} expected ${JSON.stringify(expected)}, received ${JSON.stringify(result.observations[key])}`
      );
    }
  }
  return evidence;
}

async function loadInputs(platform, profileId) {
  invariant(['android', 'ios', 'macos'].includes(platform), `unknown platform: ${platform}`);
  invariant(/^[a-z0-9_-]+$/.test(profileId), `invalid profile id: ${profileId}`);
  const profile = validateProfile(
    await loadJson(resolve(root, 'hardware/profiles', `${profileId}.json`)),
    profileId,
    platform
  );
  const spec = await loadJson(resolve(root, 'hardware/scenarios/r6-failure-states.json'));
  return { profile, spec };
}

export async function printGuide(platform, profileId, outputPath) {
  const { profile, spec } = await loadInputs(platform, profileId);
  for (const [index, scenario] of spec.scenarios.entries()) {
    console.error(`${index + 1}. ${scenario.title} (${scenario.id})`);
    scenario.instructions.forEach((instruction) => console.error(`   - ${instruction}`));
  }
  const template = `${JSON.stringify(createFailureEvidenceTemplate(spec, profile, platform), null, 2)}\n`;
  if (outputPath) {
    await writeFile(resolve(outputPath), template, { flag: 'wx' });
    console.error(`Wrote redacted evidence template to ${resolve(outputPath)}`);
  } else {
    process.stdout.write(template);
  }
}

export async function verifyFailureEvidence(platform, profileId, evidencePath) {
  invariant(evidencePath, 'an evidence path is required');
  const { profile, spec } = await loadInputs(platform, profileId);
  validateFailureEvidence(spec, profile, await loadJson(resolve(evidencePath)), platform);
  console.log(`R6 failure evidence passed: ${platform}/${profileId} | ${spec.scenarios.length} scenarios`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [, , command, platform, profileId, path] = process.argv;
  try {
    invariant(
      command === 'guide' || command === 'verify',
      'usage: hardware-r6-failure <guide|verify> <platform> <profile> [evidence.json]'
    );
    if (command === 'guide') await printGuide(platform, profileId, path);
    else await verifyFailureEvidence(platform, profileId, path);
  } catch (error) {
    console.error(`hardware-r6-failure failed: ${error.message}`);
    process.exitCode = 1;
  }
}
