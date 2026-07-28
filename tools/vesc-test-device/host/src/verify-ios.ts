import { spawn } from "node:child_process";
import { resolve } from "node:path";
import { writeEvidence } from "./evidence.js";
import { FixtureRunner } from "./runner.js";
import { loadScenario, repositoryRoot } from "./scenario.js";
import { SerialLineTransport } from "./serial.js";
import { UsbControlClient } from "./usb.js";

interface IOSSessionEvidence {
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
  mode: "ride" | "unexpected-disconnect";
  sqliteIntegrity: string;
  activeConnection: boolean;
  activeSession: boolean;
}

type VerificationMode = IOSSessionEvidence["mode"];

function argument(name: string): string {
  const index = process.argv.indexOf(name);
  const value = index < 0 ? undefined : process.argv[index + 1];
  if (!value) throw new Error(`missing required ${name}`);
  return value;
}

function optionalArgument(name: string): string | undefined {
  const index = process.argv.indexOf(name);
  return index < 0 ? undefined : process.argv[index + 1];
}

async function xcodebuild(
  deviceId: string,
  derivedData: string,
  testMethod: string,
  action: "build-for-testing" | "test-without-building",
): Promise<{ code: number; output: string }> {
  return await new Promise<{ code: number; output: string }>((resolvePromise, reject) => {
    const output: Buffer[] = [];
    let outputLength = 0;
    const child = spawn("xcodebuild", [
      "-project", resolve(repositoryRoot, "ios/float-view.xcodeproj"),
      "-scheme", "float-view-hardware",
      "-configuration", "Debug",
      "-destination", `platform=iOS,id=${deviceId}`,
      "-derivedDataPath", derivedData,
      "-allowProvisioningUpdates",
      `-only-testing:float-view-hardware-tests/T3VescTestDeviceTests/${testMethod}`,
      action,
    ], { cwd: repositoryRoot, stdio: ["ignore", "pipe", "pipe"] });
    const append = (chunk: Buffer) => {
      output.push(chunk);
      outputLength += chunk.length;
      while (outputLength > 4_000_000 && output.length > 1) {
        outputLength -= output.shift()!.length;
      }
    };
    child.stdout.on("data", append);
    child.stderr.on("data", append);
    child.once("error", reject);
    child.once("exit", (code) => {
      const text = Buffer.concat(output).toString("utf8");
      resolvePromise({ code: code ?? -1, output: text });
    });
  });
}

function decodeSession(output: string): IOSSessionEvidence {
  const matches = [...output.matchAll(/T3_IOS_RESULT:([A-Za-z0-9+/=]+)/g)];
  const encoded = matches.at(-1)?.[1];
  if (!encoded) {
    throw new Error(`iOS test passed without session evidence\n${output.slice(-20_000)}`);
  }
  return JSON.parse(Buffer.from(encoded, "base64").toString("utf8")) as IOSSessionEvidence;
}

async function main(): Promise<void> {
  const port = argument("--port");
  const deviceId = argument("--device");
  const derivedData = optionalArgument("--derived-data")
    ?? "/tmp/float-view-t3-ios-derived";
  const mode = optionalArgument("--mode") ?? "ride";
  if (mode !== "ride" && mode !== "unexpected-disconnect") {
    throw new Error(`unsupported --mode ${mode}`);
  }
  const verificationMode: VerificationMode = mode;
  const testMethod = verificationMode === "ride" ? "testRide" : "testUnexpectedDisconnect";
  const scenarioName = verificationMode === "ride"
    ? "express-thor-refloat-ride.json"
    : "express-thor-refloat-unexpected-disconnect.json";
  const scenario = await loadScenario(resolve(
    repositoryRoot,
    "conformance/scenarios/vesc-test-device",
    scenarioName,
  ));

  if (!process.argv.includes("--skip-build")) {
    process.stderr.write("iOS T3: building signed hardware-test bundle\n");
    const build = await xcodebuild(deviceId, derivedData, testMethod, "build-for-testing");
    if (build.code !== 0) {
      throw new Error(`xcodebuild build-for-testing failed (${build.code})\n`
        + build.output.slice(-30_000));
    }
    process.stderr.write("iOS T3: hardware-test bundle ready\n");
  }

  const control = new UsbControlClient(
    new SerialLineTransport(port),
    scenario.limits.commandTimeoutMs,
  );
  const runner = new FixtureRunner(scenario, control);
  let keepalive: NodeJS.Timeout | undefined;
  try {
    await runner.arm();
    process.stderr.write(`iOS T3: ESP scenario armed (${verificationMode})\n`);
    keepalive = setInterval(() => {
      void control.command("getState").catch(() => undefined);
    }, 1_000);
    const test = await xcodebuild(
      deviceId,
      derivedData,
      testMethod,
      "test-without-building",
    );
    process.stderr.write("iOS T3: XCTest returned\n");
    let runnerFailure: string | undefined;
    try {
      await runner.waitForIdle();
    } catch (error) {
      runnerFailure = error instanceof Error ? error.message : String(error);
    }
    const manifest = runner.engine.verify();
    if (runnerFailure || !manifest.passed || test.code !== 0) {
      const clientFailure = test.output.match(/T3_IOS_FAILURE:([^\r\n]+)/)?.[1];
      process.stderr.write(`${JSON.stringify({
        runnerFailure,
        clientFailure,
        xcodebuildExitCode: test.code,
        scenario: manifest,
        transcriptTail: runner.engine.transcriptSnapshot().slice(-40),
        xcodeOutputTail: test.output.slice(-20_000),
      }, null, 2)}\n`);
      throw new Error(
        runnerFailure ?? manifest.firstViolation ?? clientFailure
          ?? `iOS XCTest failed (${test.code})`,
      );
    }
    const session = decodeSession(test.output);
    const evidence = {
      ...manifest,
      platform: "ios",
      mode: verificationMode,
      session,
    };
    await writeEvidence(
      resolve(repositoryRoot, "artifacts/verification/vesc-test-device"),
      evidence,
      runner.engine.transcriptSnapshot(),
      `ios-${verificationMode}`,
    );
    process.stdout.write(`${JSON.stringify(evidence, null, 2)}\n`);
  } finally {
    if (keepalive) clearInterval(keepalive);
    await runner.stop().catch(() => undefined);
  }
}

await main();
