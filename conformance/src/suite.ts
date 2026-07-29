import assert from "node:assert/strict";
import type {
  AppState,
  BackendMethod,
  ConnectionIssueCode,
} from "../../frontend/src/lib/backend/contract.ts";
import { ConformanceClient, ConformanceError } from "./client.ts";

export interface Target {
  name: string;
  start(): Promise<{ url: string; origin: string }>;
  endpoint(): Promise<{ url: string; origin: string }>;
  background?(): Promise<void>;
  foreground?(): Promise<void>;
  stop(): Promise<void>;
}

export interface PhysicalTarget extends Target {
  platform: "android" | "ios" | "macos";
  productionBluetooth: true;
  prepareSuite?(report: (message: string) => void): Promise<void>;
  finishSuite?(report: (message: string) => void): Promise<void>;
}

export async function runConformance(target: Target) {
  const endpoint = await target.start();
  let client = await ConformanceClient.open(endpoint.url, endpoint.origin);
  try {
    const initial = await client.request("state.get", {});
    assertState(initial);
    await expectError(
      (
        client as unknown as {
          request(method: string, params: unknown): Promise<unknown>;
        }
      ).request("unknown.method", {}),
      "invalid_request",
    );

    const permission = await client.request("bluetooth.requestPermission", {});
    assert.equal(typeof permission.radio, "string");
    await client.request("bluetooth.scan", { timeoutMs: 100 });
    const scanned = await client.request("state.get", {});
    assertState(scanned);
    assert.ok(scanned.revision >= initial.revision);

    client.close();
    const replacementEndpoint = await target.endpoint();
    client = await ConformanceClient.open(
      replacementEndpoint.url,
      replacementEndpoint.origin,
    );
    const restored = await client.request("state.get", {});
    assertState(restored);
    assert.ok(restored.revision >= scanned.revision);
    assert.deepEqual(
      restored.discoveredDevices.map((device) => device.deviceId),
      scanned.discoveredDevices.map((device) => device.deviceId),
      "frontend recreation lost backend-owned state",
    );

    await exerciseProductMethods(client, restored);
    return {
      target: target.name,
      revision: (await client.request("state.get", {})).revision,
    };
  } finally {
    client.close();
    await target.stop();
  }
}

async function exerciseProductMethods(
  client: ConformanceClient,
  state: AppState,
) {
  const device = required(
    state.discoveredDevices[0],
    "scan did not create a discovered device",
  );
  const settings = await client.request("settings.update", {
    expectedRevision: state.settings.revision,
    patch: {
      automaticSessionLinkingEnabled: true,
      sessionLinkWindowMs: 7_200_000,
      themePreference: "dark",
      unitPreference: "imperial",
    },
  });
  await expectError(
    client.request("settings.update", {
      expectedRevision: state.settings.revision,
      patch: { automaticSessionLinkingEnabled: false },
    }),
    "conflict",
  );
  assert.equal(settings.automaticSessionLinkingEnabled, true);
  assert.equal(settings.themePreference, "dark");
  assert.equal(settings.unitPreference, "imperial");
  await expectError(
    (
      client as unknown as {
        request(method: string, params: unknown): Promise<unknown>;
      }
    ).request("settings.update", {
      expectedRevision: settings.revision,
      patch: { rawPacketCaptureEnabled: true },
    }),
    "invalid_request",
  );

  await createCompletedSession(client, device.deviceId);
  await createCompletedSession(client, device.deviceId);

  const prepared = await client.request("state.get", {});
  const profileDevice = required(
    prepared.savedDevices.find(
      (saved) => saved.advertisedName === device.name,
    ) ?? prepared.savedDevices[0],
    "connection did not create a saved device",
  );
  const profile = await client.request("boardProfile.get", {
    savedDeviceId: profileDevice.savedDeviceId,
  });
  assert.equal(typeof profileDevice.bluetoothIdentifier, "string");
  assert.ok(profileDevice.bluetoothIdentifier.length > 0);
  const renamed = await client.request("savedDevice.update", {
    savedDeviceId: profileDevice.savedDeviceId,
    expectedRevision: profileDevice.revision,
    customDisplayName: "Conformance Board",
  });
  assert.equal(renamed.displayName, "Conformance Board");
  assert.equal(renamed.customDisplayName, "Conformance Board");
  await expectError(
    client.request("savedDevice.update", {
      savedDeviceId: profileDevice.savedDeviceId,
      expectedRevision: profileDevice.revision,
      customDisplayName: null,
    }),
    "conflict",
  );
  const resetName = await client.request("savedDevice.update", {
    savedDeviceId: profileDevice.savedDeviceId,
    expectedRevision: renamed.revision,
    customDisplayName: null,
  });
  assert.equal(
    resetName.displayName,
    resetName.advertisedName ?? resetName.bluetoothIdentifier,
  );
  const updatedProfile = await client.request("boardProfile.update", {
    savedDeviceId: profile.savedDeviceId,
    expectedRevision: profile.revision,
    patch: { seriesCount: 18, parallelCount: 2, motor: "hypercore_xr" },
  });
  assert.equal(updatedProfile.missingFields.length, 0);
  const preferredProfile = await client.request("boardProfile.update", {
    savedDeviceId: profile.savedDeviceId,
    expectedRevision: updatedProfile.revision,
    patch: { preferredCanId: 42 },
  });
  assert.equal(preferredProfile.values.preferredCanId, 42);
  assert.equal(preferredProfile.missingFields.length, 0);
  await expectError(
    client.request("boardProfile.update", {
      savedDeviceId: profile.savedDeviceId,
      expectedRevision: preferredProfile.revision,
      patch: { preferredCanId: 254 },
    }),
    "invalid_request",
  );
  const clearedPreference = await client.request("boardProfile.update", {
    savedDeviceId: profile.savedDeviceId,
    expectedRevision: preferredProfile.revision,
    patch: { preferredCanId: null },
  });
  assert.equal(clearedPreference.values.preferredCanId, null);
  await expectError(
    client.request("boardProfile.update", {
      savedDeviceId: profile.savedDeviceId,
      expectedRevision: profile.revision,
      patch: {},
    }),
    "conflict",
  );
  await expectError(
    client.request("boardProfile.get", {
      savedDeviceId: "missing-saved-device",
    }),
    "not_found",
  );

  const current = await client.request("state.get", {});
  const session = required(
    current.recentSessions[0],
    "connections did not create a session",
  );
  const detail = await client.request("session.get", {
    sessionId: session.sessionId,
    maxPoints: 100,
  });
  assert.equal(detail.summary.sessionId, session.sessionId);
  assert.ok(detail.series.length <= 100);
  const diagnostics = await client.request("session.getDiagnostics", {
    sessionId: session.sessionId,
    centerElapsedMs: 0,
    beforeMs: 1_000,
    afterMs: 1_000,
  });
  assert.ok(diagnostics.length <= 2_000);
  await client.request("session.export", {
    sessionId: session.sessionId,
    format: "csv",
  });
  await expectError(
    client.request("session.get", { sessionId: "missing-session" }),
    "not_found",
  );

  const group = required(
    current.sessionGroups[0],
    "linked sessions did not create a session group",
  );
  await client.request("sessionGroup.unlink", {
    sessionGroupId: group.sessionGroupId,
  });
  const unlinked = await client.request("state.get", {});
  assert.ok(
    !unlinked.sessionGroups.some(
      (item) => item.sessionGroupId === group.sessionGroupId,
    ),
  );

  await client.request("session.delete", { sessionId: session.sessionId });
  const deleted = await client.request("state.get", {});
  assert.ok(
    !deleted.recentSessions.some(
      (item) => item.sessionId === session.sessionId,
    ),
  );
}

async function createCompletedSession(
  client: ConformanceClient,
  deviceId: string,
) {
  const before = await client.request("state.get", {});
  await client.request("device.connect", { deviceId });
  const connected = await client.waitForState(
    (value) =>
      value.activeConnection?.deviceId === deviceId &&
      value.activeConnection.state === "ready",
    { afterRevision: before.revision },
  );
  assertState(connected);
  assert.ok(connected.activeConnection);
  assert.equal(
    connected.activeConnection.displayName,
    connected.activeConnection.name,
  );
  assert.ok(connected.activeConnection.savedDeviceId);
  assert.ok(connected.activeConnection.activeSession);
  assert.ok(connected.activeConnection.controllerConnection);
  assert.ok(
    connected.activeConnection.controllerConnection.kind === "direct" ||
      (
        connected.activeConnection.controllerConnection.kind === "can" &&
        connected.activeConnection.controllerConnection.canId >= 0 &&
        connected.activeConnection.controllerConnection.canId <= 253
      ),
  );
  assert.deepEqual(connected.activeConnection.controllerCandidates, []);
  assert.equal(connected.activeConnection.connectionIssue, null);
  if (connected.activeConnection.latestSample) {
    assert.deepEqual(
      Object.keys(connected.activeConnection.latestSample).sort(),
      [
        "batteryCurrentA",
        "boardState",
        "controllerTempC",
        "dutyCycle",
        "elapsedMs",
        "erpm",
        "footpadState",
        "motorCurrentA",
        "motorTempC",
        "observedAt",
        "packVoltageV",
        "rideState",
        "sequence",
        "speedKph",
      ],
    );
  }
  await expectError(
    client.request("device.connect", { deviceId }),
    "invalid_state",
  );
  await client.request("device.disconnect", {});
  await client.waitForState((value) => value.activeConnection === null, {
    afterRevision: connected.revision,
  });
  const completed = await client.request("state.get", {});
  assert.ok(
    completed.recentSessions.length > before.recentSessions.length,
    "disconnect did not retain the automatic session",
  );
}

const stableConnectionIssueCodes = new Set<ConnectionIssueCode>([
  "transport_setup_failed",
  "identification_timeout",
  "no_controller_found",
  "too_many_can_devices",
  "too_many_controllers",
  "preferred_controller_unavailable",
  "controller_changed",
  "unsupported_controller",
  "unsupported_refloat",
]);

export function assertState(state: AppState) {
  assert.ok(Number.isSafeInteger(state.revision));
  assert.ok(Array.isArray(state.discoveredDevices));
  assert.ok(Array.isArray(state.savedDevices));
  assert.ok(Array.isArray(state.recentSessions));
  assert.ok(Array.isArray(state.sessionGroups));
  assert.ok(state.discoveredDevices.length <= 50);
  assert.ok(state.savedDevices.length <= 50);
  assert.ok(state.recentSessions.length <= 50);
  assert.ok(state.sessionGroups.length <= 50);
  assert.deepEqual(Object.keys(state.settings).sort(), [
    "automaticSessionLinkingEnabled",
    "idlePersistIntervalMs",
    "idleProbeIntervalMs",
    "postRideTailMs",
    "reconnectDurationMs",
    "revision",
    "ridingPersistIntervalMs",
    "sessionLinkWindowMs",
    "themePreference",
    "unitPreference",
  ]);
  assert.ok(
    ["system", "light", "dark"].includes(state.settings.themePreference),
  );
  assert.ok(["metric", "imperial"].includes(state.settings.unitPreference));

  const active = state.activeConnection;
  if (!active) return;
  assert.ok(active.controllerCandidates.length <= 16);
  assert.deepEqual(
    active.controllerCandidates.map((candidate) => candidate.canId),
    active.controllerCandidates
      .map((candidate) => candidate.canId)
      .sort((left, right) => left - right),
    "controller candidates are not sorted by CAN ID",
  );
  assert.equal(
    new Set(active.controllerCandidates.map((candidate) => candidate.canId)).size,
    active.controllerCandidates.length,
    "controller candidates contain duplicate CAN IDs",
  );
  for (const candidate of active.controllerCandidates) {
    assert.ok(
      Number.isInteger(candidate.canId) &&
        candidate.canId >= 0 &&
        candidate.canId <= 253,
    );
    assert.deepEqual(Object.keys(candidate).sort(), [
      "canId",
      "firmwareName",
      "hardwareName",
      "vescSupported",
      "vescVersion",
    ]);
    assert.ok((candidate.hardwareName?.length ?? 0) <= 80);
    assert.ok((candidate.firmwareName?.length ?? 0) <= 80);
  }
  assert.ok(
    !JSON.stringify(active.controllerCandidates).toLowerCase().includes("uuid"),
    "controller candidates exposed a private UUID",
  );

  if (active.connectionIssue) {
    assert.ok(stableConnectionIssueCodes.has(active.connectionIssue.code));
    assert.ok(active.connectionIssue.message.length > 0);
    assert.ok(active.connectionIssue.message.length <= 1_024);
  }
  if (active.state === "ready") {
    assert.ok(active.controllerConnection, "ready connection has no controller route");
    assert.ok(active.activeSession, "ready connection has no active durable session");
    assert.ok(active.firmware?.supported, "ready connection has unsupported or absent firmware");
    assert.deepEqual(active.controllerCandidates, []);
    assert.equal(active.connectionIssue, null);
  } else if (active.state === "selecting_controller") {
    assert.equal(active.controllerConnection, null);
    assert.ok(active.savedDeviceId);
    assert.ok(active.controllerCandidates.length >= 2);
    assert.equal(active.activeSession, null);
    assert.equal(active.connectionIssue, null);
  } else if (active.state === "failed") {
    assert.ok(active.connectionIssue, "failed connection has no stable issue");
    assert.deepEqual(active.controllerCandidates, []);
    assert.equal(active.activeSession, null);
  } else {
    assert.equal(active.connectionIssue, null);
    assert.deepEqual(active.controllerCandidates, []);
  }
}

async function expectError(promise: Promise<unknown>, code: string) {
  await assert.rejects(promise, (error: unknown) => {
    return error instanceof ConformanceError && error.error.code === code;
  });
}

function required<T>(value: T | null | undefined, message: string): T {
  assert.ok(value, message);
  return value;
}

export const mandatoryMethods: BackendMethod[] = [
  "state.get",
  "bluetooth.requestPermission",
  "bluetooth.scan",
  "device.connect",
  "device.selectController",
  "device.disconnect",
  "savedDevice.update",
  "settings.update",
  "boardProfile.get",
  "boardProfile.update",
  "session.get",
  "session.getDiagnostics",
  "session.export",
  "session.delete",
  "sessionGroup.unlink",
];
