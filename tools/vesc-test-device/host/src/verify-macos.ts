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

function argument(name: string): string {
  const index = process.argv.indexOf(name);
  const value = index < 0 ? undefined : process.argv[index + 1];
  if (!value) throw new Error(`missing required ${name}`);
  return value;
}

async function compileClient(output: string): Promise<void> {
  const sources = [
    "apple/Sources/FloatViewCore/Domain.swift",
    "apple/Sources/FloatViewProtocol/VescProtocol.swift",
    "apple/Sources/FloatViewBluetooth/CoreBluetoothAdapter.swift",
    "tools/vesc-test-device/host/t2_production_client.swift",
  ].map((path) => resolve(repositoryRoot, path));
  await new Promise<void>((resolvePromise, reject) => {
    const child = spawn("swiftc", [...sources, "-o", output], { stdio: ["ignore", "inherit", "inherit"] });
    child.once("exit", (code) => code === 0 ? resolvePromise() : reject(new Error(`swiftc exited ${code}`)));
    child.once("error", reject);
  });
}

async function main(): Promise<void> {
  const port = argument("--port");
  const scenarioPath = resolve(
    repositoryRoot,
    process.argv.includes("--scenario") ? argument("--scenario") : "conformance/scenarios/vesc-test-device/express-thor-refloat-identity.json",
  );
  const temporary = await mkdtemp(join(tmpdir(), "float-view-t2-"));
  const clientPath = join(temporary, "t2-production-client");
  let runner: FixtureRunner | undefined;
  let keepalive: NodeJS.Timeout | undefined;
  try {
    await compileClient(clientPath);
    const scenario = await loadScenario(scenarioPath);
    const control = new UsbControlClient(new SerialLineTransport(port), scenario.limits.commandTimeoutMs);
    runner = new FixtureRunner(scenario, control);
    await runner.arm();
    keepalive = setInterval(() => { void control.command("getState").catch(() => undefined); }, 1_000);

    const child = spawn(clientPath, [], { stdio: ["ignore", "pipe", "inherit"] });
    const exit = new Promise<number | null>((resolvePromise, reject) => {
      child.once("exit", resolvePromise);
      child.once("error", reject);
    });
    const lines = createInterface({ input: child.stdout! });
    let clientPassed = false;
    for await (const line of lines) {
      const event = JSON.parse(line) as { kind?: string; [key: string]: unknown };
      if (event.kind === "failed") throw new Error(`macOS client failed: ${line}`);
      if (event.kind === "passed") clientPassed = true;
    }
    const code = await exit;
    if (code !== 0 || !clientPassed) throw new Error(`macOS client exited ${code} without passing`);
    await runner.waitForIdle();
    const manifest = runner.engine.verify();
    if (!manifest.passed) throw new Error(`scenario failed: ${manifest.firstViolation}`);
    const evidenceDirectory = resolve(
      repositoryRoot,
      process.argv.includes("--evidence-dir")
        ? argument("--evidence-dir")
        : "artifacts/verification/vesc-test-device",
    );
    await writeEvidence(evidenceDirectory, manifest, runner.engine.transcriptSnapshot());
    process.stdout.write(`${JSON.stringify(manifest, null, 2)}\n`);
  } finally {
    if (keepalive) clearInterval(keepalive);
    if (runner) await runner.stop().catch(() => undefined);
    await rm(temporary, { recursive: true, force: true });
  }
}

await main();
