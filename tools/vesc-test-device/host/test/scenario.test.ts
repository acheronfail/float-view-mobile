import assert from "node:assert/strict";
import { writeFile, mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";
import { concatBytes, fromHex, toHex } from "../src/bytes.js";
import { ScenarioEngine } from "../src/engine.js";
import { writeEvidence } from "../src/evidence.js";
import { loadFixture } from "../src/fixtures.js";
import { matchRequest } from "../src/protocol.js";
import { loadScenario, repositoryRoot } from "../src/scenario.js";
import { decodeFrame, encodeFrame } from "../src/vesc.js";
import type { Scenario } from "../src/types.js";

const scenarioPath = resolve(
  repositoryRoot,
  "conformance/scenarios/vesc-test-device/express-thor-refloat-identity.json",
);
const rideScenarioPath = resolve(
  repositoryRoot,
  "conformance/scenarios/vesc-test-device/express-thor-refloat-ride.json",
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
    assert.equal(actions[0]!.kind, "notify");
    if (actions[0]!.kind === "notify") responseFrames.push(concatBytes(...actions[0]!.chunks));
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
  assert.equal(action.kind, "notify");
  if (action.kind !== "notify") return;
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

test("ride scenario schedule covers every occurrence and declares all cadence phases", async () => {
  const scenario = await loadScenario(rideScenarioPath);
  const telemetry = scenario.rules.find((rule) => rule.id === "refloatTelemetry");
  assert.equal(telemetry?.occurrences.minimum, 168);
  assert.deepEqual(
    telemetry?.responses?.map((response) => [
      response.fromOccurrence,
      response.toOccurrence,
      response.phase,
    ]),
    [
      [1, 3, "idle"],
      [4, 8, "riding"],
      [9, 9, "footpadWarning"],
      [10, 12, "riding"],
      [13, 163, "postRideTail"],
      [164, 168, "returnToIdle"],
    ],
  );
  assert.deepEqual(
    scenario.expectations.cadence?.map((value) => value.phase),
    ["idleProbe", "ridingProbe", "postRideTailProbe", "returnToIdleProbe"],
  );
});

test("ride-state fixtures preserve valid frames and exact state and footpad bits", async () => {
  const cases = [
    ["idle", 2, 0],
    ["riding", 3, 3],
    ["footpad-warning", 3, 1],
    ["post-ride-tail", 2, 0],
    ["return-to-idle", 2, 0],
  ] as const;
  for (const [name, state, footpad] of cases) {
    const fixture = await loadFixture("fixtures/refloat/ride-states-1.2.3.json", name);
    const payload = decodeFrame(fixture.frame);
    assert.equal(payload[9]! & 0x03, state);
    assert.equal((payload[10]! >> 6) & 0x03, footpad);
  }
});

test("ride scenario passes deterministic idle, riding, post-ride, and return cadence", async () => {
  const { engine, manifest } = await runRideScenario();
  assert.equal(manifest.passed, true);
  assert.equal(manifest.firstViolation, null);
  assert.equal(manifest.ruleCounts.refloatTelemetry, 168);
  assert.ok(manifest.cadence?.every((result) => result.passed));
  const requests = engine.transcriptSnapshot().filter(
    (entry) => entry.ruleId === "refloatTelemetry" && entry.direction === "clientToFixture",
  );
  assert.equal(requests[8]?.phase, "footpadWarning");
  assert.equal(requests[12]?.phase, "postRideTail");
  assert.equal(requests[163]?.phase, "returnToIdle");
});

test("ride scenario identifies the first post-ride cadence violation", async () => {
  const { manifest } = await runRideScenario(20);
  assert.equal(manifest.passed, false);
  assert.equal(
    manifest.firstViolation,
    "cadence refloatTelemetry occurrence 20 expected 100±80ms, observed 500ms",
  );
  assert.equal(
    manifest.cadence?.find((result) => result.phase === "postRideTailProbe")?.passed,
    false,
  );
});

test("scenario loader rejects a response schedule gap", async () => {
  const value = JSON.parse(await (await import("node:fs/promises")).readFile(rideScenarioPath, "utf8")) as {
    rules: Array<{ id: string; responses?: Array<{ fromOccurrence: number }> }>;
  };
  const telemetry = value.rules.find((rule) => rule.id === "refloatTelemetry");
  assert.ok(telemetry?.responses);
  telemetry.responses[1]!.fromOccurrence = 5;
  const directory = await mkdtemp(join(tmpdir(), "vesc-schedule-"));
  const path = join(directory, "gap.json");
  await writeFile(path, JSON.stringify(value));
  await assert.rejects(loadScenario(path), /response schedule has a gap or overlap at occurrence 4/);
});

test("engine emits an explicit host disconnect action from a scheduled occurrence", async () => {
  const loaded = await loadScenario(rideScenarioPath);
  const scenario: Scenario = {
    ...loaded,
    rules: [
      {
        id: "refloatTelemetry",
        match: { kind: "refloatTelemetry", canId: 42 },
        occurrences: { minimum: 1, maximum: 1 },
        responses: [
          {
            fromOccurrence: 1,
            toOccurrence: 1,
            phase: "unexpectedDisconnect",
            outcome: { action: "disconnect" },
          },
        ],
      },
    ],
    expectations: { orderedRuleIds: ["refloatTelemetry"] },
  };
  const [action] = await new ScenarioEngine(scenario).receiveGattWrite(
    encodeFrame(Uint8Array.of(34, 42, 36, 101, 31)),
  );
  assert.deepEqual(action, {
    kind: "disconnect",
    ruleId: "refloatTelemetry",
    phase: "unexpectedDisconnect",
  });
});

async function runRideScenario(slowOccurrence?: number) {
  let now = 0;
  const engine = new ScenarioEngine(
    await loadScenario(rideScenarioPath),
    { monotonicMilliseconds: () => now },
  );
  const identity = [
    Uint8Array.of(0),
    Uint8Array.of(62),
    Uint8Array.of(34, 42, 0),
    Uint8Array.of(34, 42, 36, 101, 0, 2, 0),
    Uint8Array.of(34, 42, 36, 101, 32),
  ];
  for (const request of identity) await engine.receiveGattWrite(encodeFrame(request));

  for (let occurrence = 1; occurrence <= 168; occurrence += 1) {
    if (occurrence >= 2 && occurrence <= 4) now += 500;
    else if (occurrence === 5 || occurrence === 165) now += 0;
    else if (occurrence >= 6 && occurrence <= 164) {
      now += occurrence === slowOccurrence ? 500 : 100;
    } else if (occurrence >= 166) now += 500;
    const [action] = await engine.receiveGattWrite(
      encodeFrame(Uint8Array.of(34, 42, 36, 101, 31)),
    );
    assert.equal(action?.kind, "notify");
  }
  return { engine, manifest: engine.verify() };
}
