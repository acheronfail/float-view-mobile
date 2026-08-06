import assert from 'node:assert/strict';
import test from 'node:test';
import {
  parseScenarioDocument,
  privateBmsIdentity,
  privateControllerIdentity,
  scenarioTemplate,
} from './scenario-document.server.ts';

test('scenario templates default to ATT-safe notification fragments', () => {
  for (const template of ['direct', 'singleExpress', 'reportedMultiController', 'directVbms'] as const) {
    assert.equal(scenarioTemplate(template).fragmentBytes, 20);
  }
  const directVbms = scenarioTemplate('directVbms');
  assert.equal(directVbms.bmsNodes[0]?.route, 'direct');
  assert.equal(directVbms.topology, 'express');
});

test('scenario documents round trip with deterministic private identities and no UUID field', () => {
  const document = scenarioTemplate('reportedMultiController');
  const parsed = parseScenarioDocument(JSON.parse(JSON.stringify(document)));
  assert.deepEqual(parsed, document);
  assert.deepEqual(
    privateControllerIdentity(parsed, 'front'),
    privateControllerIdentity({ ...parsed, controllers: [{ ...parsed.controllers[0]!, canId: 99 }] }, 'front')
  );
  assert.notDeepEqual(privateControllerIdentity(parsed, 'front'), privateControllerIdentity(parsed, 'rear'));
  const withBms = scenarioTemplate('directVbms');
  assert.deepEqual(privateBmsIdentity(withBms, 'vbms'), privateBmsIdentity(withBms, 'vbms'));
  assert.equal(JSON.stringify(parsed).toLowerCase().includes('uuid'), false);
});

test('scenario validation rejects privileged, duplicate, non-finite, oversized, and excessive input', () => {
  const base = scenarioTemplate('reportedMultiController');
  assert.throws(() => parseScenarioDocument({ ...base, controllerUuid: 'private' }), /Invalid scenario/);
  assert.throws(() => parseScenarioDocument({ ...base, schemaVersion: 2 }), /Unsupported/);
  assert.throws(
    () =>
      parseScenarioDocument({
        ...base,
        controllers: [base.controllers[0], { ...base.controllers[1]!, canId: base.controllers[0]!.canId }],
      }),
    /unique/
  );
  assert.throws(() => parseScenarioDocument({ ...base, discoveredCanIds: [12, 12] }), /duplicates/);
  assert.throws(
    () => parseScenarioDocument({ ...base, discoveredCanIds: [base.controllers[0]!.canId] }),
    /must be unique/
  );
  assert.throws(() => parseScenarioDocument({ ...base, overrides: { motorCurrentA: Number.NaN } }), /finite/);
  assert.throws(() => parseScenarioDocument({ ...base, overrides: { packageState: 'privileged' } }), /enum value/);
  assert.throws(
    () =>
      parseScenarioDocument({
        ...base,
        controllers: Array.from({ length: 17 }, (_, index) => ({
          ...base.controllers[0]!,
          alias: `controller-${index}`,
          canId: index,
        })),
      }),
    /1 through 16/
  );
  assert.throws(() => parseScenarioDocument({ ...base, namespace: 'x'.repeat(65_537) }), /65,536/);
  const bms = scenarioTemplate('directVbms').bmsNodes[0]!;
  assert.throws(
    () => parseScenarioDocument({ ...base, bmsNodes: [{ ...bms, route: 'can', canId: base.controllers[0]!.canId }] }),
    /must be unique/
  );
  assert.throws(() => parseScenarioDocument({ ...base, bmsNodes: [{ ...bms, cellCount: 51 }] }), /array counts/);
  assert.throws(
    () => parseScenarioDocument({ ...base, bmsNodes: [{ ...bms }, { ...bms, alias: 'second', canId: 43 }] }),
    /one direct VBMS/
  );
});
