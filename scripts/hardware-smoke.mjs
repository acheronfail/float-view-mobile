import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const root = resolve(import.meta.dirname, '..');

const allowedReadOnlyOperations = new Set([
  'vesc.firmwareInfo',
  'vesc.canPing',
  'vesc.forwardFirmwareInfo',
  'refloat.info',
  'refloat.realtimeFieldIds',
  'refloat.realtimeData'
]);

function invariant(condition, message) {
  if (!condition) throw new Error(message);
}

function version(value) {
  invariant(typeof value === 'string' && /^\d+\.\d+(?:\.\d+)?$/.test(value), `invalid version: ${value}`);
  const parts = value.split('.').map(Number);
  while (parts.length < 3) parts.push(0);
  return parts;
}

function compareVersion(left, right) {
  const a = version(left);
  const b = version(right);
  for (let index = 0; index < 3; index += 1) {
    if (a[index] !== b[index]) return a[index] < b[index] ? -1 : 1;
  }
  return 0;
}

export function validateProfile(profile, requestedId, platform) {
  invariant(profile.schemaVersion === 1, 'unsupported hardware profile schema');
  invariant(profile.id === requestedId, `profile id mismatch: ${profile.id}`);
  invariant(profile.supportedPlatforms?.includes(platform), `${platform} is not supported by ${profile.id}`);
  invariant(Array.isArray(profile.operations) && profile.operations.length > 0, 'profile has no operations');
  for (const operation of profile.operations) {
    invariant(operation.classification === 'read-only', `unsafe operation classification: ${operation.name}`);
    invariant(allowedReadOnlyOperations.has(operation.name), `operation is not on the read-only allowlist: ${operation.name}`);
  }
  invariant(profile.acceptance?.minimumPersistedSamples > 0, 'minimum persisted samples must be positive');
  invariant(profile.acceptance?.minimumLoggingDurationMs > 0, 'minimum logging duration must be positive');
  return profile;
}

export function validateEvidence(profile, evidence, platform) {
  const observed = evidence.observed;
  invariant(evidence.schemaVersion === 1, 'unsupported hardware evidence schema');
  invariant(evidence.profileId === profile.id, 'evidence profile mismatch');
  invariant(evidence.platform === platform, 'evidence platform mismatch');
  invariant(evidence.identityRedacted === true, 'hardware identity must be redacted');
  invariant(evidence.readOnly === true, 'hardware run was not declared read-only');
  invariant(observed && typeof observed === 'object', 'evidence observations are missing');
  invariant(profile.transport.advertisedNames.includes(observed.transportName), 'unexpected transport');
  invariant(profile.primaryControllers.includes(observed.primaryController), 'unexpected primary controller');
  invariant(compareVersion(observed.vescFirmware, profile.vescFirmware.minimum) >= 0, 'VESC firmware is too old');
  invariant(
    compareVersion(observed.vescFirmware, profile.vescFirmware.maximumExclusive) < 0,
    'VESC firmware is outside this profile'
  );
  invariant(compareVersion(observed.refloatVersion, profile.refloat.minimum) >= 0, 'Refloat version is too old');
  invariant(
    compareVersion(observed.refloatVersion, profile.refloat.maximumExclusive) < 0,
    'Refloat version is outside this profile'
  );
  invariant(
    observed.persistedSamples >= profile.acceptance.minimumPersistedSamples,
    `insufficient persisted samples: ${observed.persistedSamples}`
  );
  invariant(
    observed.committedSequence >= profile.acceptance.minimumReceivedSequence,
    `insufficient received sequence: ${observed.committedSequence}`
  );
  invariant(
    observed.loggingDurationMs >= profile.acceptance.minimumLoggingDurationMs,
    `logging duration is too short: ${observed.loggingDurationMs}ms`
  );
  invariant(observed.catalogIntegrity === profile.acceptance.requiredCatalogIntegrity, 'catalog integrity failed');
  invariant(observed.sessionIntegrity === profile.acceptance.requiredSessionIntegrity, 'session integrity failed');
  invariant(observed.sessionState === 'completed', `session is not completed: ${observed.sessionState}`);
  invariant(observed.closeReason === profile.acceptance.requiredCloseReason, 'unexpected close reason');
  invariant(
    observed.finalConnectionState === profile.acceptance.requiredFinalConnectionState,
    'connection did not reach the required terminal state'
  );
  invariant(observed.activeOsConnection === false, 'OS still reports an active connection');
  return evidence;
}

async function loadJson(path) {
  return JSON.parse(await readFile(path, 'utf8'));
}

export async function runHardwareSmoke(platform, profileId, evidencePath) {
  invariant(['android', 'ios', 'macos'].includes(platform), `unknown platform: ${platform}`);
  invariant(/^[a-z0-9_-]+$/.test(profileId), `invalid profile id: ${profileId}`);
  const profilePath = resolve(root, 'hardware/profiles', `${profileId}.json`);
  const profile = validateProfile(await loadJson(profilePath), profileId, platform);
  const selectedEvidence = evidencePath
    ? resolve(evidencePath)
    : resolve(root, 'hardware/evidence', profileId, `${platform}.json`);
  const evidence = validateEvidence(profile, await loadJson(selectedEvidence), platform);
  const observed = evidence.observed;
  console.log(
    [
      `hardware-smoke passed: ${platform}/${profileId}`,
      `${observed.primaryController} VESC ${observed.vescFirmware}`,
      `Refloat ${observed.refloatVersion}`,
      `${observed.persistedSamples} persisted samples through sequence ${observed.committedSequence}`,
      `${observed.loggingDurationMs}ms`,
      'integrity ok',
      'disconnected'
    ].join(' | ')
  );
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [, , platform, profileId, evidencePath] = process.argv;
  try {
    invariant(platform && profileId, 'usage: hardware-smoke <platform> <profile> [evidence.json]');
    await runHardwareSmoke(platform, profileId, evidencePath);
  } catch (error) {
    console.error(`hardware-smoke failed: ${error.message}`);
    process.exitCode = 1;
  }
}
