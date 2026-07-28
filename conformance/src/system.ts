export type ConformanceClient = {
  request(method: string, params: unknown): Promise<unknown>;
  close(): void;
};

export type ConformanceScenarioResult = {
  name: string;
  durationMs: number;
};

export type ConformanceClientFactory = () => Promise<ConformanceClient>;

type ErrorEnvelope = {
  code: number;
  data: {
    kind: string;
    recoverable: boolean;
  };
};

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function asRecord(value: unknown, label: string): Record<string, unknown> {
  assert(
    !!value && typeof value === "object" && !Array.isArray(value),
    `${label} must be an object`,
  );
  return value as Record<string, unknown>;
}

function errorEnvelope(value: unknown): ErrorEnvelope | null {
  if (!value || typeof value !== "object") return null;
  const error = value as Record<string, unknown>;
  const data = error.data;
  if (!data || typeof data !== "object") return null;
  const details = data as Record<string, unknown>;
  if (
    typeof error.code !== "number" ||
    typeof details.kind !== "string" ||
    typeof details.recoverable !== "boolean"
  ) {
    return null;
  }
  return {
    code: error.code,
    data: {
      kind: details.kind,
      recoverable: details.recoverable,
    },
  };
}

async function expectApiError(
  promise: Promise<unknown>,
  expectedCode: number,
  expectedKind: string,
): Promise<void> {
  try {
    await promise;
    throw new Error(
      `Expected ${expectedKind} error, but the request succeeded`,
    );
  } catch (error) {
    const envelope = errorEnvelope(error);
    assert(
      envelope,
      `Expected a structured API error, received ${String(error)}`,
    );
    assert(
      envelope.code === expectedCode,
      `Expected error code ${expectedCode}, got ${envelope.code}`,
    );
    assert(
      envelope.data.kind === expectedKind,
      `Expected error kind ${expectedKind}, got ${envelope.data.kind}`,
    );
  }
}

async function withClient<T>(
  createClient: ConformanceClientFactory,
  run: (client: ConformanceClient) => Promise<T>,
): Promise<T> {
  const client = await createClient();
  try {
    return await run(client);
  } finally {
    client.close();
  }
}

export async function runSystemConformance(
  createClient: ConformanceClientFactory,
  options: { includeProductScenarios?: boolean } = {},
): Promise<ConformanceScenarioResult[]> {
  const scenarios: Array<[string, () => Promise<void>]> = [
    [
      "requires system.hello before other methods",
      () =>
        withClient(createClient, (client) =>
          expectApiError(
            client.request("system.getSnapshot", {}),
            -32000,
            "invalid_state",
          ),
        ),
    ],
    [
      "rejects an incompatible API major version",
      () =>
        withClient(createClient, (client) =>
          expectApiError(
            client.request("system.hello", {
              client: { name: "float-view-conformance", version: "1.0.0" },
              api: { major: 2, minMinor: 0, maxMinor: 0 },
            }),
            -32001,
            "unsupported",
          ),
        ),
    ],
    [
      "negotiates v1.0 and returns an authoritative snapshot",
      () =>
        withClient(createClient, async (client) => {
          const hello = asRecord(
            await client.request("system.hello", {
              client: { name: "float-view-conformance", version: "1.0.0" },
              api: { major: 1, minMinor: 0, maxMinor: 0 },
            }),
            "system.hello result",
          );
          const version = asRecord(hello.api, "selected API version");
          assert(
            version.major === 1 && version.minor === 0,
            "Backend must select API v1.0",
          );
          const backend = asRecord(hello.backend, "backend info");
          assert(
            typeof backend.platform === "string",
            "Backend platform must be present",
          );
          assert(
            Array.isArray(hello.capabilities),
            "Capabilities must be an array",
          );

          const initial = asRecord(hello.snapshot, "initial snapshot");
          const refreshed = asRecord(
            await client.request("system.getSnapshot", {}),
            "system.getSnapshot result",
          );
          assert(
            typeof initial.revision === "number" &&
              refreshed.revision === initial.revision,
            "Snapshot revision must be stable without intervening state changes",
          );
          assert(
            Array.isArray(refreshed.knownDevices),
            "Snapshot knownDevices must be an array",
          );
          assert(
            refreshed.activeConnection === null ||
              typeof refreshed.activeConnection === "object",
            "Snapshot activeConnection must be a device or null",
          );
          assert(
            Array.isArray(refreshed.telemetrySubscriptions),
            "Snapshot telemetrySubscriptions must be an array",
          );
          assert(
            Array.isArray(refreshed.loggingJobs),
            "Snapshot loggingJobs must be an array",
          );
        }),
    ],
    [
      "rejects unknown methods with a stable unsupported error",
      () =>
        withClient(createClient, async (client) => {
          await client.request("system.hello", {
            client: { name: "float-view-conformance", version: "1.0.0" },
            api: { major: 1, minMinor: 0, maxMinor: 0 },
          });
          await expectApiError(
            client.request("system.doesNotExist", {}),
            -32601,
            "unsupported",
          );
        }),
    ],
    [
      "discovers connects logs and disconnects as one durable workflow",
      () =>
        withClient(createClient, async (client) => {
          await client.request("system.hello", {
            client: { name: "float-view-conformance", version: "1.0.0" },
            api: { major: 1, minMinor: 0, maxMinor: 0 },
          });
          await client.request("discovery.start", { timeoutMs: 1_000 });
          const discovered = asRecord(
            await client.request("system.getSnapshot", {}),
            "discovery snapshot",
          );
          assert(
            Array.isArray(discovered.knownDevices) &&
              discovered.knownDevices.length > 0,
            "Discovery must produce at least one fake board",
          );
          const first = asRecord(
            discovered.knownDevices[0],
            "discovered board",
          );
          const connected = asRecord(
            await client.request("device.connect", {
              deviceId: first.deviceId,
              timeoutMs: 5_000,
            }),
            "connected board",
          );
          assert(
            connected.state === "ready",
            "Connected board must become ready",
          );

          const active = asRecord(
            await client.request("system.getSnapshot", {}),
            "active snapshot",
          );
          assert(
            asRecord(active.activeConnection, "active connection").deviceId ===
              first.deviceId,
            "Snapshot must identify the singleton active board",
          );
          assert(
            Array.isArray(active.loggingJobs) &&
              active.loggingJobs.length === 1,
            "Ready must create exactly one logging job",
          );

          const firmware = asRecord(
            await client.request("vesc.getFirmwareInfo", {}),
            "firmware info",
          );
          assert(
            firmware.vescVersion === "6.06" &&
              firmware.packageName === "refloat" &&
              firmware.packageVersion === "1.2",
            "Fixture must report the initial supported firmware profile",
          );

          await client.request("device.disconnect", {});
          const stopped = asRecord(
            await client.request("system.getSnapshot", {}),
            "disconnected snapshot",
          );
          assert(
            stopped.activeConnection === null &&
              Array.isArray(stopped.loggingJobs) &&
              stopped.loggingJobs.length === 0,
            "Explicit disconnect must close logging before clearing the connection",
          );
          const page = asRecord(
            await client.request("sessions.list", { limit: 20 }),
            "session page",
          );
          assert(
            Array.isArray(page.items) && page.items.length === 1,
            "Workflow must leave one reviewable session",
          );
          const session = asRecord(page.items[0], "completed session");
          assert(
            session.state === "completed" && Number(session.sampleCount) > 0,
            "Disconnected session must be completed with committed samples",
          );
          const overview = (await client.request("sessions.querySeries", {
            sessionId: session.sessionId,
            startElapsedMs: 0,
            endElapsedMs: Number(session.totalElapsedMs),
            fields: ["packVoltageV", "dutyCycle", "speedKph"],
            maxPoints: 20,
          })) as unknown[];
          assert(
            overview.length > 0 && overview.length <= 20,
            "Review query must return a non-empty bounded series",
          );
          const diagnostic = (await client.request(
            "sessions.getDiagnosticWindow",
            {
              sessionId: session.sessionId,
              centerElapsedMs: Math.floor(Number(session.totalElapsedMs) / 2),
              beforeMs: 5_000,
              afterMs: 5_000,
              fields: ["packVoltageV", "dutyCycle", "footpadState"],
            },
          )) as unknown[];
          assert(
            diagnostic.length > 0,
            "Diagnostic query must return lossless samples around the requested timestamp",
          );
        }),
    ],
    [
      "persists profile and settings updates with revision checks",
      () =>
        withClient(createClient, async (client) => {
          await client.request("system.hello", {
            client: { name: "float-view-conformance", version: "1.0.0" },
            api: { major: 1, minMinor: 0, maxMinor: 0 },
          });
          await client.request("discovery.start", {});
          await client.request("device.connect", {
            deviceId: "fixture-thor300",
          });
          const saved = (await client.request(
            "savedDevices.list",
            {},
          )) as unknown[];
          const savedDevice = asRecord(saved[0], "saved board");
          const profile = asRecord(
            await client.request("boardProfiles.get", {
              savedDeviceId: savedDevice.savedDeviceId,
            }),
            "board profile",
          );
          const updated = asRecord(
            await client.request("boardProfiles.update", {
              savedDeviceId: savedDevice.savedDeviceId,
              expectedRevision: profile.revision,
              patch: { seriesCount: 18, motor: "hypercore_xr" },
            }),
            "updated profile",
          );
          assert(
            updated.revision === 1 &&
              asRecord(updated.values, "profile values").seriesCount === 18,
            "Profile update must preserve values and advance the revision",
          );
          await expectApiError(
            client.request("boardProfiles.update", {
              savedDeviceId: savedDevice.savedDeviceId,
              expectedRevision: 0,
              patch: { seriesCount: 20 },
            }),
            -32000,
            "invalid_state",
          );

          const settings = asRecord(
            await client.request("settings.get", {}),
            "settings",
          );
          const changed = asRecord(
            await client.request("settings.update", {
              expectedRevision: settings.revision,
              patch: { automaticSessionLinkingEnabled: true },
            }),
            "updated settings",
          );
          assert(
            changed.automaticSessionLinkingEnabled === true &&
              changed.revision === 1,
            "Settings update must be revisioned",
          );
        }),
    ],
  ];

  const selected =
    options.includeProductScenarios === false
      ? scenarios.slice(0, 4)
      : scenarios;
  const results: ConformanceScenarioResult[] = [];
  for (const [name, scenario] of selected) {
    const startedAt = performance.now();
    await scenario();
    results.push({ name, durationMs: performance.now() - startedAt });
  }
  return results;
}

export async function runSessionReadConformance(
  client: ConformanceClient,
): Promise<ScenarioResult> {
  const startedAt = performance.now();
  await client.request("system.hello", {
    client: { name: "float-view-conformance", version: "1.0.0" },
    api: { major: 1, minMinor: 0, maxMinor: 0 },
  });
  const page = asRecord(
    await client.request("sessions.list", { limit: 1 }),
    "session page",
  );
  assert(
    Array.isArray(page.items) && page.items.length === 1,
    "Seeded session must be listed",
  );
  const session = asRecord(page.items[0], "seeded session");
  assert(
    Number(session.sampleCount) > 0,
    "Seeded session must contain durable samples",
  );
  const overview = (await client.request("sessions.querySeries", {
    sessionId: session.sessionId,
    startElapsedMs: 0,
    endElapsedMs: Number(session.totalElapsedMs),
    fields: ["packVoltageV", "dutyCycle", "speedKph"],
    maxPoints: 12,
  })) as unknown[];
  assert(
    overview.length > 0 && overview.length <= 12,
    "Overview must respect maxPoints",
  );
  const diagnostic = (await client.request("sessions.getDiagnosticWindow", {
    sessionId: session.sessionId,
    centerElapsedMs: Math.floor(Number(session.totalElapsedMs) / 2),
    beforeMs: 2_000,
    afterMs: 2_000,
    fields: ["packVoltageV", "dutyCycle", "footpadState"],
  })) as unknown[];
  assert(
    diagnostic.length > 0,
    "Diagnostic window must return canonical samples",
  );
  return {
    name: "lists durable sessions and serves bounded review and diagnostic samples",
    durationMs: performance.now() - startedAt,
  };
}

export async function runBoardProfileConformance(
  client: ConformanceClient,
  savedDeviceId: string,
): Promise<ConformanceScenarioResult> {
  const started = performance.now();
  const initial = asRecord(
    await client.request("boardProfiles.get", { savedDeviceId }),
    "initial board profile",
  );
  assert(
    initial.revision === 0,
    "A newly created profile must start at revision 0",
  );
  assert(
    Array.isArray(initial.missingFields) &&
      initial.missingFields.includes("seriesCount") &&
      initial.missingFields.includes("parallelCount") &&
      initial.missingFields.includes("motor"),
    "A partial profile must advertise its initial missing fields",
  );
  await expectApiError(
    client.request("boardProfiles.update", {
      savedDeviceId,
      expectedRevision: 0,
      patch: { seriesCount: 0 },
    }),
    -32602,
    "protocol_error",
  );

  const first = asRecord(
    await client.request("boardProfiles.update", {
      savedDeviceId,
      expectedRevision: 0,
      patch: { seriesCount: 18, motor: "hypercore_xr" },
    }),
    "first Board Profile update",
  );
  assert(
    first.revision === 1,
    "A successful profile update must advance revision",
  );
  const firstValues = asRecord(first.values, "first profile values");
  assert(
    firstValues.seriesCount === 18 && firstValues.motor === "hypercore_xr",
    "A profile update must preserve typed patch values",
  );
  await expectApiError(
    client.request("boardProfiles.update", {
      savedDeviceId,
      expectedRevision: 0,
      patch: { seriesCount: 20 },
    }),
    -32000,
    "invalid_state",
  );

  const second = asRecord(
    await client.request("boardProfiles.update", {
      savedDeviceId,
      expectedRevision: 1,
      patch: { parallelCount: 2 },
    }),
    "second Board Profile update",
  );
  assert(
    second.revision === 2 &&
      Array.isArray(second.missingFields) &&
      second.missingFields.length === 0,
    "Completing required profile fields must clear the completeness warning",
  );
  const fetched = asRecord(
    await client.request("boardProfiles.get", { savedDeviceId }),
    "persisted Board Profile",
  );
  assert(
    fetched.revision === 2 &&
      asRecord(fetched.values, "persisted profile values").seriesCount === 18,
    "Board Profile reads must return the merged persisted revision",
  );
  const saved = (await client.request("savedDevices.list", {})) as unknown[];
  const matching = saved
    .map((value) => asRecord(value, "saved board"))
    .find((value) => value.savedDeviceId === savedDeviceId);
  assert(
    matching?.profileComplete === true,
    "Saved-device completeness must reflect the persisted Board Profile",
  );
  await expectApiError(
    client.request("boardProfiles.get", { savedDeviceId: "missing-board" }),
    -32004,
    "device_not_found",
  );

  return {
    name: "persists local Board Profiles with optimistic revision checks",
    durationMs: performance.now() - started,
  };
}
