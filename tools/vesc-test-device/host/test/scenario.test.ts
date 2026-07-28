import assert from "node:assert/strict";
import { writeFile, mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";
import { concatBytes, fromHex, toHex } from "../src/bytes.js";
import { ScenarioEngine } from "../src/engine.js";
import { writeEvidence } from "../src/evidence.js";
import { matchRequest } from "../src/protocol.js";
import { loadScenario, repositoryRoot } from "../src/scenario.js";
import { decodeFrame, encodeFrame } from "../src/vesc.js";

const scenarioPath = resolve(
  repositoryRoot,
  "conformance/scenarios/vesc-test-device/express-thor-refloat-identity.json",
);

test("scenario loader validates the committed first identity slice", async () => {
  const scenario = await loadScenario(scenarioPath);
  assert.equal(scenario.topology.primaryCanId, 42);
  assert.equal(scenario.rules.length, 6);
});

test("scenario loader identifies schema errors with a useful path", async () => {
  const directory = await mkdtemp(join(tmpdir(), "vesc-scenario-"));
  const path = join(directory, "invalid.json");
  await writeFile(path, JSON.stringify({ schemaVersion: 1, id: "bad" }));
  await assert.rejects(loadScenario(path), /invalid scenario:.*required property/i);
});

test("read-only matcher recognizes direct and CAN-forwarded requests", () => {
  assert.deepEqual(matchRequest(Uint8Array.of(0)), { disposition: "readOnly", kind: "transportFirmware" });
  assert.deepEqual(matchRequest(Uint8Array.of(62)), { disposition: "readOnly", kind: "canPing" });
  assert.deepEqual(matchRequest(Uint8Array.of(34, 42, 0)), {
    disposition: "readOnly", kind: "primaryFirmware", canId: 42,
  });
  assert.deepEqual(matchRequest(Uint8Array.of(34, 42, 36, 101, 32)), {
    disposition: "readOnly", kind: "refloatTelemetryIds", canId: 42,
  });
});

test("matcher rejects known motion commands and unknown commands", () => {
  assert.equal(matchRequest(Uint8Array.of(5, 0, 0, 0, 1)).disposition, "mutating");
  assert.equal(matchRequest(Uint8Array.of(200)).disposition, "unknown");
});

test("engine runs identity, CAN, Refloat layout and telemetry deterministically", async () => {
  const scenario = await loadScenario(scenarioPath);
  const engine = new ScenarioEngine(scenario, { monotonicMilliseconds: () => 123 });
  const requests = [
    Uint8Array.of(0),
    Uint8Array.of(62),
    Uint8Array.of(34, 42, 0),
    Uint8Array.of(34, 42, 36, 101, 0, 2, 0),
    Uint8Array.of(34, 42, 36, 101, 32),
    Uint8Array.of(34, 42, 36, 101, 31),
  ];
  const responseFrames: Uint8Array[] = [];
  for (const request of requests) {
    const actions = await engine.receiveGattWrite(encodeFrame(request));
    assert.equal(actions.length, 1);
    responseFrames.push(concatBytes(...actions[0]!.chunks));
  }
  assert.deepEqual([...decodeFrame(responseFrames[0]!)].slice(0, 3), [0, 6, 5]);
  assert.deepEqual([...decodeFrame(responseFrames[1]!)], [62, 42]);
  assert.equal(toHex(responseFrames[2]!), "020b00060654484f52333030002ad803");
  assert.deepEqual([...decodeFrame(responseFrames[3]!)].slice(0, 4), [36, 101, 0, 2]);
  assert.deepEqual([...decodeFrame(responseFrames[4]!)].slice(0, 3), [36, 101, 32]);
  assert.deepEqual([...decodeFrame(responseFrames[5]!)].slice(0, 3), [36, 101, 31]);
  assert.deepEqual(engine.verify(), {
    schemaVersion: 1,
    scenarioId: "express-thor-refloat-identity",
    passed: true,
    firstViolation: null,
    ruleCounts: {
      transportFirmware: 1,
      canPing: 1,
      primaryFirmware: 1,
      refloatInfo: 1,
      refloatTelemetryIds: 1,
      refloatTelemetry: 1,
    },
    transcriptEntries: 6,
    transcriptTruncated: false,
  });
});

test("engine accepts client writes fragmented across arbitrary GATT boundaries", async () => {
  const engine = new ScenarioEngine(await loadScenario(scenarioPath));
  const request = encodeFrame(Uint8Array.of(0));
  assert.deepEqual(await engine.receiveGattWrite(request.slice(0, 1)), []);
  assert.deepEqual(await engine.receiveGattWrite(request.slice(1, 4)), []);
  assert.equal((await engine.receiveGattWrite(request.slice(4))).length, 1);
});

test("engine rejects mutating commands and records the first violated expectation", async () => {
  const engine = new ScenarioEngine(await loadScenario(scenarioPath));
  assert.deepEqual(await engine.receiveGattWrite(encodeFrame(Uint8Array.of(5, 0, 0, 0, 1))), []);
  const manifest = engine.verify();
  assert.equal(manifest.passed, false);
  assert.equal(manifest.firstViolation, "mutating command 5 rejected");
  assert.equal(engine.transcriptSnapshot()[0]?.outcome, "rejected");
});

test("engine reports the first out-of-order rule and excessive duplicate", async () => {
  const engine = new ScenarioEngine(await loadScenario(scenarioPath));
  await engine.receiveGattWrite(encodeFrame(Uint8Array.of(62)));
  await engine.receiveGattWrite(encodeFrame(Uint8Array.of(62)));
  const manifest = engine.verify();
  assert.equal(manifest.firstViolation, "expected ordered rule transportFirmware before canPing");
  assert.equal(manifest.ruleCounts.canPing, 2);
});

test("fixture chunk plans reconstruct exact frames", async () => {
  const engine = new ScenarioEngine(await loadScenario(scenarioPath));
  const [action] = await engine.receiveGattWrite(encodeFrame(Uint8Array.of(34, 42, 36, 101, 32)));
  assert.ok(action);
  assert.equal(action.chunks.length, 21);
  assert.equal(
    toHex(concatBytes(...action.chunks)),
    JSON.parse(await (await import("node:fs/promises")).readFile(
      resolve(repositoryRoot, "fixtures/refloat/rtdata-1.2.3-android-hardware.json"),
      "utf8",
    )).cases[1].frameHex,
  );
});

test("bounded transcript and manifest can be written as machine-readable evidence", async () => {
  const directory = await mkdtemp(join(tmpdir(), "vesc-evidence-"));
  const engine = new ScenarioEngine(await loadScenario(scenarioPath));
  await engine.receiveGattWrite(encodeFrame(Uint8Array.of(5)));
  const manifest = engine.verify();
  const paths = await writeEvidence(directory, manifest, engine.transcriptSnapshot());
  const written = JSON.parse(
    await (await import("node:fs/promises")).readFile(paths.manifestPath, "utf8"),
  ) as { firstViolation: string };
  assert.equal(written.firstViolation, "mutating command 5 rejected");
  await assert.rejects(
    writeEvidence(directory, manifest, engine.transcriptSnapshot()),
    /EEXIST/,
  );
});
