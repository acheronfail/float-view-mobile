import assert from "node:assert/strict";
import type {
  AppState,
  BackendMethod,
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
    assert.ok(scanned.revision >= initial.revision);

    client.close();
    const replacementEndpoint = await target.endpoint();
    client = await ConformanceClient.open(
      replacementEndpoint.url,
      replacementEndpoint.origin,
    );
    const restored = await client.request("state.get", {});
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
  const updatedProfile = await client.request("boardProfile.update", {
    savedDeviceId: profile.savedDeviceId,
    expectedRevision: profile.revision,
    patch: { seriesCount: 18, parallelCount: 2, motor: "hypercore_xr" },
  });
  assert.equal(updatedProfile.missingFields.length, 0);
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
    (value) => value.activeConnection?.deviceId === deviceId,
    { afterRevision: before.revision },
  );
  assert.ok(connected.activeConnection);
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

function assertState(state: AppState) {
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
  ]);
  assert.ok(
    ["system", "light", "dark"].includes(state.settings.themePreference),
  );
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
  "device.disconnect",
  "settings.update",
  "boardProfile.get",
  "boardProfile.update",
  "session.get",
  "session.getDiagnostics",
  "session.export",
  "session.delete",
  "sessionGroup.unlink",
];
