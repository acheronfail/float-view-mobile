import assert from 'node:assert/strict';
import { runHardwareSmoke, validateProfile } from './hardware-smoke.mjs';

const validProfile = {
  schemaVersion: 1,
  id: 'fixture',
  supportedPlatforms: ['ios'],
  operations: [{ name: 'vesc.firmwareInfo', classification: 'read-only' }],
  acceptance: { minimumPersistedSamples: 1, minimumLoggingDurationMs: 1 }
};

assert.equal(validateProfile(validProfile, 'fixture', 'ios'), validProfile);
assert.throws(
  () =>
    validateProfile(
      {
        ...validProfile,
        operations: [{ name: 'vesc.writeConfiguration', classification: 'configuration-write' }]
      },
      'fixture',
      'ios'
    ),
  /unsafe operation/
);
assert.throws(() => validateProfile(validProfile, 'fixture', 'windows'), /not supported/);

for (const platform of ['android', 'ios', 'macos']) {
  await runHardwareSmoke(platform, 'vesc-6_06-refloat-1_2');
}

console.log('hardware-smoke safety and recorded-evidence tests passed');
