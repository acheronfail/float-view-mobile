import assert from 'node:assert/strict';
import test from 'node:test';
import {
  bmsIdentity,
  controllerIdentity,
  parseScenarioDocument,
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

test('scenario documents round trip with explicit canonical UUID identities', () => {
  const document = scenarioTemplate('reportedMultiController');
  const parsed = parseScenarioDocument(JSON.parse(JSON.stringify(document)));
  assert.deepEqual(parsed, document);
  assert.deepEqual(
    controllerIdentity(parsed, 'front'),
    controllerIdentity({ ...parsed, controllers: [{ ...parsed.controllers[0]!, canId: 99 }] }, 'front')
  );
  assert.notDeepEqual(controllerIdentity(parsed, 'front'), controllerIdentity(parsed, 'rear'));
  const withBms = scenarioTemplate('directVbms');
  const bmsAlias = withBms.bmsNodes[0]!.alias;
  assert.deepEqual(bmsIdentity(withBms, bmsAlias), bmsIdentity(withBms, bmsAlias));
  assert.match(parsed.controllers[0]!.uuid, /^[0-9a-f]{24}$/);
  assert.equal(JSON.stringify(parsed).includes(parsed.controllers[0]!.uuid), true);
});

test('scenario validation rejects privileged, duplicate, non-finite, oversized, and excessive input', () => {
  const base = scenarioTemplate('reportedMultiController');
  assert.throws(
    () => parseScenarioDocument({ ...base, controllers: [{ ...base.controllers[0]!, uuid: 'not-a-uuid' }, base.controllers[1]!] }),
    /canonical/
  );
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
          uuid: index.toString(16).padStart(24, '0'),
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
    () =>
      parseScenarioDocument({
        ...base,
        bmsNodes: [{ ...bms }, { ...bms, alias: 'second', uuid: 'abcdefabcdef000000000000', canId: 43 }],
      }),
    /one direct VBMS/
  );
});
