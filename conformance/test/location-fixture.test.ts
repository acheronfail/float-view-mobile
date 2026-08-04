import assert from 'node:assert/strict';
import test from 'node:test';
import { defaultLocationFixture, DeterministicLocationFixture } from '../fixture/location.ts';

test('deterministic location fixture models cadence, sparse observations, and permission transitions', () => {
  const fixture = new DeterministicLocationFixture({
    cadenceMs: 2_000,
    initialStatus: 'notDetermined',
    observations: [
      { elapsedMs: 2_000, latitude: 1, longitude: 2, accuracyM: 3 },
      { elapsedMs: 4_000, latitude: 4, longitude: 5, accuracyM: null },
      { elapsedMs: 10_000, latitude: 6, longitude: 7, accuracyM: 8 },
    ],
    permissionTransitions: [{ observationIndex: 1, status: 'authorized', backgroundCapture: 'available' }],
  });
  const emitted: number[] = [];
  assert.equal(
    fixture.start((observation) => emitted.push(observation.elapsedMs)),
    true
  );
  assert.deepEqual(
    fixture.emitAll().map((observation) => observation.elapsedMs),
    [4_000, 10_000]
  );
  assert.deepEqual(emitted, [4_000, 10_000]);
  assert.deepEqual(
    fixture.snapshot().permissionTransitions.map((item) => item.status),
    ['authorized']
  );
  assert.equal(fixture.cadenceMs, 2_000);
});

test('fixture records interruptions and bounded update counts', () => {
  const fixture = defaultLocationFixture({ interruptAtObservation: 1 });
  assert.equal(
    fixture.start(() => undefined),
    true
  );
  fixture.emitAll();
  const snapshot = fixture.snapshot();
  assert.equal(snapshot.attemptedCount, 3);
  assert.equal(snapshot.emittedCount, 2);
  assert.deepEqual(snapshot.interruptions, [2_000]);
  assert.deepEqual(snapshot.updateElapsedMs, [1_000, 20_000]);
  assert.equal(fixture.stop(), true);
  assert.equal(fixture.stop(), false);
});
