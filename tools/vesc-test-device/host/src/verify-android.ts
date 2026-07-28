import { execFile } from "node:child_process";
import { resolve } from "node:path";
import { promisify } from "node:util";
import { physicalRunLabel, writeEvidence } from "./evidence.js";
import { FixtureRunner } from "./runner.js";
import {
  physicalHostSequenceSeed,
  resolveAndroidSerial,
  resolveEspPort,
} from "./lab.js";
import { loadScenario, repositoryRoot } from "./scenario.js";
import { SerialLineTransport } from "./serial.js";
import { UsbControlClient } from "./usb.js";

interface AndroidSessionEvidence {
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
  sawUnexpectedDisconnectReason: boolean;
  closeReason: string;
  mode: "ride" | "unexpected-disconnect" | "background" | "same-process-soak";
  sqliteIntegrity: string;
  activeConnection: boolean;
  activeSession: boolean;
  foregroundServiceObserved: boolean;
  frontendDestroyed: boolean;
  frontendRecreated: boolean;
  processId: number;
  residentKiB: number;
  openFileDescriptors: number;
  cycleCount: number;
  residentKiBMinimum: number;
  residentKiBMaximum: number;
  openFileDescriptorsMinimum: number;
  openFileDescriptorsMaximum: number;
}

interface AndroidResourceEvidence {
  residentKiB: number;
  openFileDescriptors: number;
}

type VerificationMode = AndroidSessionEvidence["mode"];

function optionalArgument(name: string): string | undefined {
  const index = process.argv.indexOf(name);
  return index < 0 ? undefined : process.argv[index + 1];
}

async function command(
  executable: string,
  args: readonly string[],
  cwd = repositoryRoot,
): Promise<string> {
  try {
    const result = await execFileAsync(executable, [...args], {
      cwd,
      maxBuffer: 10 * 1024 * 1024,
    });
    return result.stdout;
  } catch (error) {
    const failure = error as Error & { stdout?: string; stderr?: string };
    throw new Error(
      `${executable} failed\n${failure.stderr ?? ""}${failure.stdout ?? ""}`,
      { cause: error },
    );
  }
}

async function prepareDevice(serial: string): Promise<void> {
  process.stderr.write("android T3: building and installing instrumentation\n");
  await command(
    "./gradlew",
    ["--no-daemon", ":app:installDebug", ":app:installDebugAndroidTest"],
    resolve(repositoryRoot, "android"),
  );
  await command("adb", ["-s", serial, "shell", "pm", "clear", APPLICATION_ID]);
  await command("adb", [
    "-s",
    serial,
    "shell",
    "pm",
    "grant",
    APPLICATION_ID,
    "android.permission.ACCESS_FINE_LOCATION",
  ]);
  process.stderr.write("android T3: device prepared\n");
}

async function main(): Promise<void> {
  const port = optionalArgument("--port") ?? (await resolveEspPort());
  const serial = optionalArgument("--serial") ?? (await resolveAndroidSerial());
  const mode = optionalArgument("--mode") ?? "ride";
  if (
    mode !== "ride" &&
    mode !== "unexpected-disconnect" &&
    mode !== "background" &&
    mode !== "same-process-soak"
  ) {
    throw new Error(`unsupported --mode ${mode}`);
  }
  const verificationMode: VerificationMode = mode;
  const scenarioName = {
    ride: "express-thor-refloat-ride.json",
    "unexpected-disconnect": "express-thor-refloat-unexpected-disconnect.json",
    background: "express-thor-refloat-background.json",
    "same-process-soak": "express-thor-refloat-same-process-soak.json",
  }[verificationMode];
  const scenario = await loadScenario(
    resolve(
      repositoryRoot,
      "conformance/scenarios/vesc-test-device",
      scenarioName,
    ),
  );

  if (!process.argv.includes("--skip-prepare")) {
    await prepareDevice(serial);
  }
  const control = new UsbControlClient(
    new SerialLineTransport(port),
    scenario.limits.commandTimeoutMs,
    undefined,
    physicalHostSequenceSeed(),
  );
  const runner = new FixtureRunner(scenario, control);
  let keepalive: NodeJS.Timeout | undefined;
  try {
    await runner.arm();
    process.stderr.write(
      `android T3: ESP scenario armed (${verificationMode})\n`,
    );
    keepalive = setInterval(() => {
      void control.command("getState").catch(() => undefined);
    }, 1_000);
    const instrumentation = await command("adb", [
      "-s",
      serial,
      "shell",
      "am",
      "instrument",
      "-w",
      "-r",
      "-e",
      "class",
      `${APPLICATION_ID}.T3VescTestDeviceTest`,
      "-e",
      "mode",
      verificationMode,
      `${APPLICATION_ID}.test/androidx.test.runner.AndroidJUnitRunner`,
    ]);
    process.stderr.write("android T3: instrumentation returned\n");
    let runnerFailure: string | undefined;
    try {
      await runner.waitForIdle();
    } catch (error) {
      runnerFailure = error instanceof Error ? error.message : String(error);
    }
    const manifest = runner.engine.verify();
    if (
      runnerFailure ||
      !manifest.passed ||
      !instrumentation.includes("OK (1 test)")
    ) {
      process.stderr.write(
        `${JSON.stringify(
          {
            runnerFailure,
            instrumentation,
            scenario: manifest,
            transcriptTail: runner.engine.transcriptSnapshot().slice(-40),
          },
          null,
          2,
        )}\n`,
      );
      throw new Error(
        runnerFailure ??
          manifest.firstViolation ??
          "Android instrumentation did not report one passing test",
      );
    }
    const rawSession = await command("adb", [
      "-s",
      serial,
      "exec-out",
      "run-as",
      APPLICATION_ID,
      "cat",
      "cache/t3-android-result.json",
    ]);
    const session = JSON.parse(rawSession) as AndroidSessionEvidence;
    const resources: AndroidResourceEvidence = {
      residentKiB: session.residentKiB,
      openFileDescriptors: session.openFileDescriptors,
    };
    process.stderr.write("android T3: durable evidence collected\n");
    const evidence = {
      ...manifest,
      platform: "android",
      mode: verificationMode,
      session,
      resources,
    };
    await writeEvidence(
      resolve(repositoryRoot, "artifacts/verification/vesc-test-device"),
      evidence,
      runner.engine.transcriptSnapshot(),
      physicalRunLabel("android", verificationMode),
    );
    process.stdout.write(`${JSON.stringify(evidence, null, 2)}\n`);
  } finally {
    if (keepalive) clearInterval(keepalive);
    await runner.stop().catch(() => undefined);
  }
}

const APPLICATION_ID = "fail.acheron.floatview";
const execFileAsync = promisify(execFile);

await main();
