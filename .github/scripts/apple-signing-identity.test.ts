import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  chmodSync,
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join } from "node:path";
import { afterEach, test } from "node:test";

const root = new URL("../../", import.meta.url).pathname;
const directories: string[] = [];
afterEach(() => {
  for (const path of directories.splice(0))
    rmSync(path, { recursive: true, force: true });
});
const fakeTool = `#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const tool = path.basename(process.argv[1]);
const args = process.argv.slice(2);
const op = args.slice(0, 2).join(' ');
const log = (value) => fs.appendFileSync(process.env.FAKE_LOG, JSON.stringify(value) + '\\n');
log({ tool, operation: args[0] });
if (tool === 'security') {
  if (args[0] === 'import' && fs.readFileSync(args[1], 'utf8') !== 'test') process.exit(1);
  if (args[0] === 'list-keychains' && args.length === 3) console.log('  "/tmp/original.keychain-db"');
  if (args[0] === 'find-identity') console.log('  1) AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA "' + process.env.FAKE_IDENTITY + '"');
  if (args[0] === 'find-identity' && args.includes('basic') && process.env.FAKE_INSTALLER_IDENTITY) console.log('  2) BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB "' + process.env.FAKE_INSTALLER_IDENTITY + '"');
  process.exit(0);
}
if (tool === 'probe') {
  console.log(JSON.stringify({ identity: process.env.CODE_SIGN_IDENTITY, hasKeychain: !!process.env.CODE_SIGN_KEYCHAIN,
    installer: process.env.MAC_INSTALLER_IDENTITY, installerKeychain: process.env.MAC_INSTALLER_KEYCHAIN,
    leakedCertificateSecret: !!process.env.APPLE_CODE_SIGN_P12_BASE64 || !!process.env.APPLE_CODE_SIGN_P12_PASSWORD || !!process.env.MAC_INSTALLER_P12_BASE64 || !!process.env.MAC_INSTALLER_P12_PASSWORD }));
  process.exit(0);
}
throw new Error('Unexpected fake tool invocation: ' + tool + ' ' + op);
`;
function fixture() {
  const path = mkdtempSync(join(tmpdir(), "float-view-signing-test-"));
  directories.push(path);
  const bin = join(path, "bin");
  mkdirSync(bin);
  for (const name of ["security", "probe"]) {
    const file = join(bin, name);
    writeFileSync(file, fakeTool);
    chmodSync(file, 0o755);
  }
  const log = join(path, "operations.jsonl");
  const env = {
    ...process.env,
    PATH: bin + delimiter + process.env.PATH,
    RUNNER_TEMP: path,
    APPLE_TEAM_ID: "TESTTEAM01",
    GITHUB_ACTIONS: "false",
    FAKE_LOG: log,
  };
  const operations = () =>
    existsSync(log)
      ? readFileSync(log, "utf8")
          .trim()
          .split("\n")
          .map((line) => JSON.parse(line))
      : [];
  return { path, env, operations };
}
for (const [identity, accepted] of [
  ["Developer ID Application: Test Person (TESTTEAM01)", true],
  ["Developer ID Application: Other Person (OTHERTEAM1)", false],
  ["Float View Code Signing", false],
] as const) {
  test(`CI import ${accepted ? "accepts selected team" : "rejects"}: ${identity}`, () => {
    const item = fixture();
    const result = spawnSync(
      "bash",
      [join(root, "backends/apple/with-apple-signing-identity.sh"), "probe"],
      {
        env: {
          ...item.env,
          GITHUB_ACTIONS: "true",
          RUNNER_ENVIRONMENT: "github-hosted",
          RUNNER_OS: "macOS",
          APPLE_CODE_SIGN_P12_BASE64: "dGVzdA==",
          APPLE_CODE_SIGN_P12_PASSWORD: "certificate-secret",
          FAKE_IDENTITY: identity,
        },
        encoding: "utf8",
        timeout: 10_000,
      },
    );
    assert.equal(result.status === 0, accepted, result.stderr);
    assert.equal(
      item.operations().some((x) => x.operation === "add-trusted-cert"),
      false,
    );
    assert.equal(
      item.operations().some((x) => x.operation === "delete-keychain"),
      true,
    );
    if (accepted)
      assert.deepEqual(JSON.parse(result.stdout), {
        identity: "A".repeat(40),
        hasKeychain: true,
        leakedCertificateSecret: false,
      });
  });
}
test("CI import refuses a developer workstation", () => {
  const item = fixture();
  const result = spawnSync(
    "bash",
    [join(root, "backends/apple/with-apple-signing-identity.sh"), "probe"],
    {
      env: item.env,
      encoding: "utf8",
      timeout: 10_000,
    },
  );
  assert.notEqual(result.status, 0);
  assert.deepEqual(item.operations(), []);
});

test("CI import rejects malformed base64 before importing a certificate", () => {
  const item = fixture();
  const result = spawnSync(
    "bash",
    [join(root, "backends/apple/with-apple-signing-identity.sh"), "probe"],
    {
      env: {
        ...item.env,
        GITHUB_ACTIONS: "true",
        RUNNER_ENVIRONMENT: "github-hosted",
        RUNNER_OS: "macOS",
        APPLE_CODE_SIGN_P12_BASE64: "!!!!",
        APPLE_CODE_SIGN_P12_PASSWORD: "certificate-secret",
      },
      encoding: "utf8",
      timeout: 10_000,
    },
  );
  assert.notEqual(result.status, 0);
  assert.equal(item.operations().some((x) => x.operation === "import"), false);
});

for (const [identity, accepted] of [
  ["Apple Distribution: Example Person (TESTTEAM01)", true],
  ["Apple Distribution: Other Person (OTHERTEAM1)", false],
  ["Developer ID Application: Example Person (TESTTEAM01)", false],
] as const) {
  test(`iOS CI selects only the publishing distribution identity: ${identity}`, () => {
    const item = fixture();
    const result = spawnSync("bash", [join(root, "backends/apple/with-apple-signing-identity.sh"), "probe"], {
      env: { ...item.env, GITHUB_ACTIONS: "true", RUNNER_ENVIRONMENT: "github-hosted", RUNNER_OS: "macOS",
        APPLE_SIGNING_CERTIFICATE_KIND: "Apple Distribution", APPLE_CODE_SIGN_P12_BASE64: "dGVzdA==",
        APPLE_CODE_SIGN_P12_PASSWORD: "certificate-secret", FAKE_IDENTITY: identity },
      encoding: "utf8", timeout: 10_000,
    });
    assert.equal(result.status === 0, accepted, result.stderr);
    assert.ok(item.operations().some(x => x.operation === "delete-keychain"));
    if (accepted) assert.equal(JSON.parse(result.stdout).leakedCertificateSecret, false);
  });
}

for (const [installer, accepted] of [
  ["3rd Party Mac Developer Installer: Example Person (TESTTEAM01)", true],
  ["3rd Party Mac Developer Installer: Other Person (OTHERTEAM1)", false],
  ["Developer ID Installer: Example Person (TESTTEAM01)", false],
] as const) {
  test(`Mac CI confines both signing identities and rejects wrong installer: ${installer}`, () => {
    const item = fixture();
    const result = spawnSync("bash", [join(root, "backends/apple/with-apple-signing-identity.sh"), "probe"], {
      env: { ...item.env, GITHUB_ACTIONS: "true", RUNNER_ENVIRONMENT: "github-hosted", RUNNER_OS: "macOS",
        APPLE_SIGNING_CERTIFICATE_KIND: "Apple Distribution", APPLE_CODE_SIGN_P12_BASE64: "dGVzdA==",
        APPLE_CODE_SIGN_P12_PASSWORD: "app-secret", FAKE_IDENTITY: "Apple Distribution: Person (TESTTEAM01)",
        MAC_INSTALLER_P12_BASE64: "dGVzdA==", MAC_INSTALLER_P12_PASSWORD: "installer-secret",
        FAKE_INSTALLER_IDENTITY: installer },
      encoding: "utf8", timeout: 10_000,
    });
    assert.equal(result.status === 0, accepted, result.stderr);
    assert.ok(item.operations().some(x => x.operation === "delete-keychain"));
    if (accepted) {
      const output = JSON.parse(result.stdout);
      assert.equal(output.installer, installer);
      assert.ok(output.installerKeychain);
      assert.equal(output.leakedCertificateSecret, false);
      assert.equal(item.operations().filter(x => x.operation === "import").length, 2);
    }
  });
}
