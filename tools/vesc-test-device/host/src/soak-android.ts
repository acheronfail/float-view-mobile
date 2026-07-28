import { execFile } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { promisify } from "node:util";
import { physicalRunLabel } from "./evidence.js";
import { repositoryRoot } from "./scenario.js";

const execFileAsync = promisify(execFile);

interface CycleEvidence {
  passed: boolean;
  scenarioId: string;
  session: {
    sessionIntegrity: string;
    sqliteIntegrity: string;
    activeConnection: boolean;
    activeSession: boolean;
    closeReason: string;
    processId: number;
  };
  resources: {
    residentKiB: number;
    openFileDescriptors: number;
  };
}

const requestedCycles = Number(optionalArgument("--cycles") ?? "20");
if (
  !Number.isInteger(requestedCycles) ||
  requestedCycles < 1 ||
  requestedCycles > 100
) {
  throw new Error("--cycles must be an integer from 1 through 100");
}

const cycles: Array<CycleEvidence & { cycle: number; mode: string }> = [];
for (let cycle = 1; cycle <= requestedCycles; cycle += 1) {
  const mode = cycle % 5 === 0 ? "background" : "ride";
  process.stderr.write(
    `android R8 soak: cycle ${cycle}/${requestedCycles} (${mode})\n`,
  );
  const args = [
    "--import",
    "tsx",
    "src/verify-android.ts",
    "--mode",
    mode,
    ...(cycle === 1 ? [] : ["--skip-prepare"]),
  ];
  const result = await execFileAsync(process.execPath, args, {
    cwd: resolve(repositoryRoot, "tools/vesc-test-device/host"),
    maxBuffer: 20 * 1024 * 1024,
  });
  process.stderr.write(result.stderr);
  const evidence = JSON.parse(result.stdout) as CycleEvidence;
  assertCycle(evidence, cycle);
  cycles.push({ ...evidence, cycle, mode });
}

const rssValues = cycles.map((cycle) => cycle.resources.residentKiB);
const fdValues = cycles.map((cycle) => cycle.resources.openFileDescriptors);
const resourceBounds = {
  residentKiBMinimum: Math.min(...rssValues),
  residentKiBMaximum: Math.max(...rssValues),
  residentKiBSpread: Math.max(...rssValues) - Math.min(...rssValues),
  openFileDescriptorsMinimum: Math.min(...fdValues),
  openFileDescriptorsMaximum: Math.max(...fdValues),
  openFileDescriptorsSpread: Math.max(...fdValues) - Math.min(...fdValues),
  processIds: [...new Set(cycles.map((cycle) => cycle.session.processId))],
  acceptedResidentKiBSpread: 96 * 1024,
  acceptedFileDescriptorSpread: 48,
};
if (
  resourceBounds.residentKiBSpread > resourceBounds.acceptedResidentKiBSpread
) {
  throw new Error(
    `resident memory spread exceeded bound: ${resourceBounds.residentKiBSpread} KiB`,
  );
}
if (
  resourceBounds.openFileDescriptorsSpread >
  resourceBounds.acceptedFileDescriptorSpread
) {
  throw new Error(
    `file descriptor spread exceeded bound: ${resourceBounds.openFileDescriptorsSpread}`,
  );
}

const aggregate = {
  schemaVersion: 1,
  milestone: "R8",
  platform: "android",
  fixture: "express-thor-refloat",
  passed: true,
  cycleCount: requestedCycles,
  backgroundCycles: cycles.filter((cycle) => cycle.mode === "background")
    .length,
  resourceBounds,
  cycles: cycles.map((cycle) => ({
    cycle: cycle.cycle,
    mode: cycle.mode,
    scenarioId: cycle.scenarioId,
    session: cycle.session,
    resources: cycle.resources,
  })),
};
const directory = resolve(
  repositoryRoot,
  "artifacts/verification/vesc-test-device",
);
await mkdir(directory, { recursive: true });
const path = resolve(
  directory,
  `${physicalRunLabel("android", "r8-soak")}.json`,
);
await writeFile(path, `${JSON.stringify(aggregate, null, 2)}\n`, {
  flag: "wx",
});
process.stdout.write(
  `${JSON.stringify({ ...aggregate, evidencePath: path }, null, 2)}\n`,
);

function optionalArgument(name: string): string | undefined {
  const index = process.argv.indexOf(name);
  return index < 0 ? undefined : process.argv[index + 1];
}

function assertCycle(evidence: CycleEvidence, cycle: number): void {
  if (
    !evidence.passed ||
    evidence.session.sessionIntegrity !== "complete" ||
    evidence.session.sqliteIntegrity !== "ok" ||
    evidence.session.activeConnection ||
    evidence.session.activeSession ||
    evidence.session.closeReason !== "explicit_disconnect"
  ) {
    throw new Error(
      `cycle ${cycle} did not reach a clean durable terminal state`,
    );
  }
  if (
    evidence.resources.residentKiB <= 0 ||
    evidence.resources.openFileDescriptors <= 0
  ) {
    throw new Error(`cycle ${cycle} did not report valid resource counters`);
  }
}
