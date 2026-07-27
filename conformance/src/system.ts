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
            Array.isArray(refreshed.devices),
            "Snapshot devices must be an array",
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
  ];

  const results: ConformanceScenarioResult[] = [];
  for (const [name, scenario] of scenarios) {
    const startedAt = performance.now();
    await scenario();
    results.push({ name, durationMs: performance.now() - startedAt });
  }
  return results;
}
