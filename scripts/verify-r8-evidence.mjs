import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const profileId = process.argv[2];
if (!profileId) {
  throw new Error("Usage: node scripts/verify-r8-evidence.mjs <profile>");
}

const root = process.cwd();
const profile = JSON.parse(
  await readFile(resolve(root, "hardware/profiles", `${profileId}.json`), "utf8"),
);
assert.equal(profile.id, profileId, "hardware profile ID does not match its file");

const evidencePath = resolve(root, "hardware/evidence/esp-r8-development.json");
const evidence = JSON.parse(await readFile(evidencePath, "utf8"));
assert.equal(evidence.schemaVersion, 1);
assert.equal(evidence.android.cycles, 20);
assert.equal(evidence.android.rideCycles + evidence.android.backgroundCycles, 20);
assert.equal(evidence.android.allSessionsComplete, true);
assert.equal(evidence.android.allSQLiteIntegrity, "ok");
assert.equal(evidence.android.allTerminalConnectionsCleared, true);
assert.equal(evidence.android.sameProcessSoak.cycles, 20);
assert.equal(evidence.android.sameProcessSoak.telemetrySamples, 3360);
assert.equal(evidence.android.sameProcessSoak.allSessionsComplete, true);
assert.equal(evidence.android.sameProcessSoak.allSQLiteIntegrity, "ok");
assert.equal(evidence.android.sameProcessSoak.terminalConnectionCleared, true);
assert.equal(evidence.android.sameProcessSoak.terminalSessionCleared, true);
assert.equal(evidence.android.sameProcessSoak.cadencePassed, true);
assert.equal(evidence.android.sameProcessSoak.transcriptTruncated, false);
assert.ok(
  evidence.android.sameProcessSoak.residentKiBRange[1] -
    evidence.android.sameProcessSoak.residentKiBRange[0] <
    64 * 1024,
  "Android same-process resident-memory spread exceeds 64 MiB",
);
assert.ok(
  evidence.android.sameProcessSoak.openFileDescriptorRange[1] -
    evidence.android.sameProcessSoak.openFileDescriptorRange[0] <=
    4,
  "Android same-process file-descriptor spread exceeds four",
);
for (const platform of ["ios", "macos"]) {
  assert.equal(evidence[platform].ride.telemetrySamples, 168);
  assert.equal(evidence[platform].ride.closeReason, "explicit_disconnect");
  assert.equal(evidence[platform].ride.sqliteIntegrity, "ok");
  assert.equal(
    evidence[platform].unexpectedDisconnect.closeReason,
    "unexpected_disconnect",
  );
  assert.equal(evidence[platform].unexpectedDisconnect.sqliteIntegrity, "ok");
}

if (evidence.remainingAcceptance.length > 0) {
  process.stderr.write(
    `R8 evidence is structurally valid but incomplete:\n${evidence.remainingAcceptance
      .map((item) => `- ${item}`)
      .join("\n")}\n`,
  );
  process.exitCode = 1;
} else {
  process.stdout.write(`R8 evidence complete for ${profileId}\n`);
}
