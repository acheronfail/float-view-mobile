import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import {
  createFailureEvidenceTemplate,
  validateFailureEvidence
} from './hardware-r6-failure.mjs';

const root = resolve(import.meta.dirname, '..');
const spec = JSON.parse(
  await readFile(resolve(root, 'hardware/scenarios/r6-failure-states.json'), 'utf8')
);
const profile = JSON.parse(
  await readFile(resolve(root, 'hardware/profiles/vesc-6_06-refloat-1_2.json'), 'utf8')
);

for (const platform of ['android', 'ios', 'macos']) {
  const evidence = createFailureEvidenceTemplate(spec, profile, platform);
  evidence.recordedAt = '2026-07-28T00:00:00Z';
  evidence.hostQualification = `${platform} deterministic fixture`;
  for (const scenario of spec.scenarios) {
    evidence.scenarios[scenario.id].passed = true;
    evidence.scenarios[scenario.id].observations = structuredClone(scenario.expected);
  }
  assert.equal(validateFailureEvidence(spec, profile, evidence, platform), evidence);

  const incomplete = structuredClone(evidence);
  incomplete.scenarios.radio_cycle.passed = false;
  assert.throws(
    () => validateFailureEvidence(spec, profile, incomplete, platform),
    /scenario did not pass/
  );

  const sensitive = structuredClone(evidence);
  sensitive.deviceId = 'forbidden';
  assert.throws(
    () => validateFailureEvidence(spec, profile, sensitive, platform),
    /sensitive field is forbidden/
  );
}

console.log('R6 guided failure-evidence safety and validation tests passed');
