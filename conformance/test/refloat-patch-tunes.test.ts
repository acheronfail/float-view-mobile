import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { makeDirectTarget } from '../targets/direct.ts';
import { InteractiveTelemetrySource } from '../fixture/telemetry.ts';
import { ConformanceClient, ConformanceError } from '../src/client.ts';
import type {
  RefloatTune,
  RefloatConfigurationField,
  RefloatConfigurationSchemaPage,
} from '../../frontend/src/lib/backend/contract.ts';

for (const versions of [
  ['1.3.0', '1.3.1'],
  ['1.3.1', '1.3.0'],
] as const) {
  test(
    `direct: saved ${versions[0]} Tunes apply and overwrite on ${versions[1]} with target readback`,
    { timeout: 60000 },
    async () => {
      const storageRoot = mkdtempSync(join(tmpdir(), 'float-view-patch-tunes-'));
      let tune: RefloatTune | undefined;
      try {
        for (const refloatVersion of versions) {
          const telemetry = new InteractiveTelemetrySource();
          telemetry.setScene('idle');
          const target = makeDirectTarget({ telemetry, refloatVersion, storageRoot, onSpeech() {} });
          let client: ConformanceClient | undefined;
          try {
            const endpoint = await target.start();
            client = await ConformanceClient.open(endpoint.url, endpoint.origin);
            const state = await client.request('state.get', {});
            await client.request('settings.update', {
              expectedRevision: state.settings.revision,
              patch: { safetyDisclaimerAccepted: true },
            });
            await client.request('bluetooth.scan', { timeoutMs: 10000 });
            const scan = await client.waitForState((s) => s.discoveredDevices.length > 0);
            await client.request('device.connect', {
              deviceId: scan.discoveredDevices[0]!.deviceId,
              trackingMode: 'guest',
            });
            await client.waitForState(
              (s) =>
                s.activeConnection?.state === 'ready' && s.activeConnection.refloatConfiguration?.currentLoaded === true
            );
            const current = await client.request('refloat.configuration.read', { source: 'current', refresh: true });
            assert.equal(current.identity.packageVersion, refloatVersion);
            if (!tune) {
              const fields: RefloatConfigurationField[] = [];
              let cursor: string | null = null;
              do {
                const page: RefloatConfigurationSchemaPage = await client.request('refloat.configuration.schema', {
                  cursor,
                });
                fields.push(...page.fields);
                cursor = page.nextCursor;
              } while (cursor);
              const values = Object.fromEntries(
                fields.filter((field) => field.tunable).map((field) => [field.name, current.values[field.name]!])
              );
              values.kp = { type: 'double', value: 21 };
              tune = await client.request('refloat.tune.create', {
                name: 'Patch tune',
                description: null,
                identity: current.identity,
                sourceRevision: current.revision,
                values,
              });
            } else {
              assert.notEqual(tune.identity.schemaFingerprint, current.identity.schemaFingerprint);
              const listed = await client.request('refloat.tune.list', { identity: current.identity, limit: 1 });
              assert.equal(listed.items[0]?.tuneId, tune.tuneId);
              assert.deepEqual(await client.request('refloat.tune.get', { tuneId: tune.tuneId }), tune);
              const applied = await client.request('refloat.tune.apply', {
                tuneId: tune.tuneId,
                expectedRevision: tune.revision,
                baseRevision: current.revision,
              });
              assert.deepEqual(applied.identity, current.identity);
              assert.deepEqual(applied.values.kp, { type: 'double', value: 21 });
              await client.request('refloat.configuration.write', {
                packageVersion: applied.identity.packageVersion,
                schemaFingerprint: applied.identity.schemaFingerprint,
                wireSignature: applied.identity.wireSignature,
                baseRevision: applied.revision,
                values: { ...applied.values, kp: { type: 'double', value: 22 } },
              });
              // Overwrite from a fresh target read and retain the original source identity.
              const fresh = await client.request('refloat.configuration.read', { source: 'current', refresh: true });
              const values = Object.fromEntries(Object.keys(tune.values).map((key) => [key, fresh.values[key]!]));
              const overwritten = await client.request('refloat.tune.update', {
                tuneId: tune.tuneId,
                expectedRevision: tune.revision,
                name: tune.name,
                description: tune.description,
                values,
              });
              assert.deepEqual(overwritten.identity, tune.identity);
              assert.deepEqual(overwritten.values.kp, { type: 'double', value: 22 });
              assert.equal(overwritten.revision, tune.revision + 1);
              await assert.rejects(
                client.request('refloat.tune.apply', {
                  tuneId: tune.tuneId,
                  expectedRevision: tune.revision,
                  baseRevision: fresh.revision,
                }),
                (error: unknown) => error instanceof ConformanceError && error.error.code === 'conflict'
              );
            }
            await client.request('device.disconnect', {});
          } finally {
            client?.close();
            await target.stop();
          }
        }
      } finally {
        rmSync(storageRoot, { recursive: true, force: true });
      }
    }
  );
}
