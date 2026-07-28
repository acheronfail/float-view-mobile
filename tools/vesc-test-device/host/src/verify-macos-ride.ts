import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import { writeEvidence } from "./evidence.js";
import { FixtureRunner } from "./runner.js";
import { loadScenario, repositoryRoot } from "./scenario.js";
import { SerialLineTransport } from "./serial.js";
import { UsbControlClient } from "./usb.js";

interface SessionEvidence {
  sessionId: string;
  sessionState: string;
  sessionIntegrity: string;
  sampleCount: number;
  ridingElapsedMs: number;
  totalElapsedMs: number;
  telemetryCount: number;
  ridingTelemetryCount: number;
  idleTelemetryCount: number;
  sawFootpadWarning: boolean;
  sqliteIntegrity: string;
  activeConnection: boolean;
  activeSession: boolean;
}

function argument(name: string): string {
  const index = process.argv.indexOf(name);
  const value = index < 0 ? undefined : process.argv[index + 1];
  if (!value) throw new Error(`missing required ${name}`);
  return value;
}

async function compileClient(output: string): Promise<void> {
  const sources = [
    "apple/Sources/FloatViewCore/Domain.swift",
    "apple/Sources/FloatViewCore/AdaptiveLogging.swift",
    "apple/Sources/FloatViewCore/ApplicationService.swift",
    "apple/Sources/FloatViewProtocol/VescProtocol.swift",
    "apple/Sources/FloatViewBluetooth/CoreBluetoothAdapter.swift",
    "apple/Sources/FloatViewLogging/LogSchema.swift",
    "apple/Sources/FloatViewLogging/SQLiteLogStore.swift",
    "tools/vesc-test-device/host/t3_macos_session_client.swift",
  ].map((path) => resolve(repositoryRoot, path));
  await new Promise<void>((resolvePromise, reject) => {
    const child = spawn(
      "swiftc",
      [
        "-module-cache-path", join(temporaryModuleCache(output)),
        ...sources,
        "-lsqlite3",
        "-o", output,
      ],
      { stdio: ["ignore", "inherit", "inherit"] },
    );
    child.once("exit", (code) => code === 0
      ? resolvePromise()
      : reject(new Error(`swiftc exited ${code}`)));
    child.once("error", reject);
  });
}

function temporaryModuleCache(output: string): string {
  return `${output}-module-cache`;
}

async function main(): Promise<void> {
  const port = argument("--port");
  const scenarioPath = resolve(
    repositoryRoot,
    "conformance/scenarios/vesc-test-device/express-thor-refloat-ride.json",
  );
  const temporary = await mkdtemp(join(tmpdir(), "float-view-t3-"));
  const clientPath = join(temporary, "t3-macos-session-client");
  const logRoot = join(temporary, "session");
  let runner: FixtureRunner | undefined;
  let keepalive: NodeJS.Timeout | undefined;
  try {
    await compileClient(clientPath);
    const scenario = await loadScenario(scenarioPath);
    const control = new UsbControlClient(
      new SerialLineTransport(port),
      scenario.limits.commandTimeoutMs,
    );
    runner = new FixtureRunner(scenario, control);
    await runner.arm();
    keepalive = setInterval(() => {
      void control.command("getState").catch(() => undefined);
    }, 1_000);

    const child = spawn(
      clientPath,
      [logRoot, resolve(repositoryRoot, "logging/sql")],
      { stdio: ["ignore", "pipe", "inherit"] },
    );
    const exit = new Promise<number | null>((resolvePromise, reject) => {
      child.once("exit", resolvePromise);
      child.once("error", reject);
    });
    const lines = createInterface({ input: child.stdout! });
    let session: SessionEvidence | undefined;
    let clientFailure: string | undefined;
    for await (const line of lines) {
      const event = JSON.parse(line) as { kind?: string; code?: string; detail?: string };
      if (event.kind === "failed") {
        clientFailure = `macOS session client failed: ${event.code} ${event.detail ?? ""}`.trim();
      }
      if (event.kind === "passed") session = event as unknown as SessionEvidence;
    }
    const code = await exit;
    await runner.waitForIdle();
    const manifest = runner.engine.verify();
    if (clientFailure || code !== 0 || !session) {
      process.stderr.write(`${JSON.stringify({ clientFailure, code, scenario: manifest }, null, 2)}\n`);
      throw new Error(clientFailure ?? `macOS session client exited ${code} without passing`);
    }
    if (!manifest.passed) throw new Error(`scenario failed: ${manifest.firstViolation}`);
    const evidence = { ...manifest, platform: "macos", session };
    const evidenceDirectory = resolve(
      repositoryRoot,
      "artifacts/verification/vesc-test-device",
    );
    await writeEvidence(
      evidenceDirectory,
      evidence,
      runner.engine.transcriptSnapshot(),
    );
    process.stdout.write(`${JSON.stringify(evidence, null, 2)}\n`);
  } finally {
    if (keepalive) clearInterval(keepalive);
    if (runner) await runner.stop().catch(() => undefined);
    await rm(temporary, { recursive: true, force: true });
  }
}

await main();
