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
log({ tool, operation: tool === 'xcrun' ? op : args[0], passwordInEnvironment: !!process.env.APPLE_NOTARY_PASSWORD });
if (tool === 'codesign') process.exit(process.env.FAKE_SIGNATURE_FAILURE ? 1 : 0);
if (tool === 'xcrun') {
  if (op === 'notarytool store-credentials') process.exit(0);
  if (op === 'notarytool submit') {
    if (process.env.FAKE_STATUS === 'malformed') { console.log('bad json'); process.exit(1); }
    console.log(JSON.stringify({ id: '12345678-1234-1234-1234-123456789012', status: process.env.FAKE_STATUS }));
    process.exit(process.env.FAKE_STATUS === 'In Progress' ? 69 : 0);
  }
  if (op === 'notarytool log') {
    fs.writeFileSync(args[args.length - 1], JSON.stringify({ issues: [{ code: 4000, message: 'Rejected test signature' }] }));
    process.exit(0);
  }
  if (op === 'stapler staple') {
    if (process.env.FAKE_STAPLE_FAILURE) process.exit(1);
    fs.writeFileSync(path.join(args[2], '.ticket'), 'ticket');
    process.exit(0);
  }
  if (op === 'stapler validate') process.exit(fs.existsSync(path.join(args[2], '.ticket')) ? 0 : 1);
}
if (tool === 'ditto') {
  const source = args[args.length - 2];
  fs.writeFileSync(args[args.length - 1], fs.existsSync(path.join(source, '.ticket')) ? 'archive-with-ticket' : 'archive-without-ticket');
  process.exit(0);
}
if (tool === 'spctl') process.exit(process.env.FAKE_GATEKEEPER_FAILURE ? 1 : 0);
if (tool === 'unzip') process.exit(0);
if (tool === 'security') {
  if (args[0] === 'import' && fs.readFileSync(args[1], 'utf8') !== 'test') process.exit(1);
  if (args[0] === 'list-keychains' && args.length === 3) console.log('  "/tmp/original.keychain-db"');
  if (args[0] === 'find-identity') console.log('  1) AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA "' + process.env.FAKE_IDENTITY + '"');
  process.exit(0);
}
if (tool === 'probe') {
  console.log(JSON.stringify({ identity: process.env.CODE_SIGN_IDENTITY, hasKeychain: !!process.env.CODE_SIGN_KEYCHAIN,
    leakedCertificateSecret: !!process.env.APPLE_CODE_SIGN_P12_BASE64 || !!process.env.APPLE_CODE_SIGN_P12_PASSWORD }));
  process.exit(0);
}
throw new Error('Unexpected fake tool invocation: ' + tool + ' ' + op);
`;
function fixture() {
  const path = mkdtempSync(join(tmpdir(), "float-view-notary-test-"));
  directories.push(path);
  const bin = join(path, "bin");
  mkdirSync(bin);
  for (const name of [
    "xcrun",
    "codesign",
    "ditto",
    "spctl",
    "unzip",
    "security",
    "probe",
  ]) {
    const file = join(bin, name);
    writeFileSync(file, fakeTool);
    chmodSync(file, 0o755);
  }
  const app = join(path, "FloatView.app");
  mkdirSync(app);
  const output = join(path, "FloatView.zip");
  writeFileSync(output, "previous-successful-download");
  const log = join(path, "operations.jsonl");
  const env = {
    ...process.env,
    PATH: bin + delimiter + process.env.PATH,
    RUNNER_TEMP: path,
    APPLE_TEAM_ID: "TESTTEAM01",
    GITHUB_ACTIONS: "false",
    FAKE_LOG: log,
    FAKE_STATUS: "Accepted",
  };
  const operations = () =>
    existsSync(log)
      ? readFileSync(log, "utf8")
          .trim()
          .split("\n")
          .map((line) => JSON.parse(line))
      : [];
  return { path, app, output, env, operations };
}
function runNotary(item: ReturnType<typeof fixture>, extra = {}) {
  return spawnSync(
    "bash",
    [
      join(root, "backends/apple/notarize-macos-release.sh"),
      item.app,
      item.output,
    ],
    {
      env: { ...item.env, ...extra },
      encoding: "utf8",
      timeout: 10_000,
    },
  );
}

test("notarization staples before rebuilding the downloadable ZIP", () => {
  const item = fixture();
  const result = runNotary(item);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(readFileSync(item.output, "utf8"), "archive-with-ticket");
  const operations = item.operations().map((x) => `${x.tool} ${x.operation}`);
  assert.ok(
    operations.indexOf("xcrun stapler validate") <
      operations.indexOf("spctl --assess"),
  );
  assert.equal(operations.filter((x) => x === "ditto -c").length, 2);
  assert.equal(
    item
      .operations()
      .some((x) => x.operation === "notarytool store-credentials"),
    false,
  );
});
for (const status of ["Invalid", "Rejected", "In Progress", "malformed"]) {
  test(`notarization ${status} preserves the previous download and never staples`, () => {
    const item = fixture();
    const result = runNotary(item, { FAKE_STATUS: status });
    assert.notEqual(result.status, 0);
    assert.equal(
      readFileSync(item.output, "utf8"),
      "previous-successful-download",
    );
    assert.equal(
      item.operations().some((x) => x.operation === "stapler staple"),
      false,
    );
  });
}
test("stapling failure does not replace the previous download", () => {
  const item = fixture();
  assert.notEqual(runNotary(item, { FAKE_STAPLE_FAILURE: "1" }).status, 0);
  assert.equal(
    readFileSync(item.output, "utf8"),
    "previous-successful-download",
  );
});
test("invalid signing identity fails before submission", () => {
  const item = fixture();
  assert.notEqual(runNotary(item, { FAKE_SIGNATURE_FAILURE: "1" }).status, 0);
  assert.equal(
    item.operations().some((x) => x.operation === "notarytool submit"),
    false,
  );
});
test("Gatekeeper failure preserves the previous download after stapling", () => {
  const item = fixture();
  assert.notEqual(runNotary(item, { FAKE_GATEKEEPER_FAILURE: "1" }).status, 0);
  assert.equal(
    readFileSync(item.output, "utf8"),
    "previous-successful-download",
  );
  assert.equal(item.operations().filter((x) => x.tool === "ditto").length, 1);
});
test("CI stores notarization credentials only in its disposable keychain", () => {
  const item = fixture();
  const result = runNotary(item, {
    GITHUB_ACTIONS: "true",
    CODE_SIGN_KEYCHAIN: join(item.path, "temporary.keychain-db"),
    APPLE_NOTARY_APPLE_ID: "test@example.invalid",
    APPLE_NOTARY_PASSWORD: "secret-sentinel",
  });
  assert.equal(result.status, 0, result.stderr);
  const calls = item.operations();
  assert.equal(
    calls.find((x) => x.operation === "notarytool store-credentials")
      .passwordInEnvironment,
    true,
  );
  assert.equal(
    calls.find((x) => x.operation === "notarytool submit")
      .passwordInEnvironment,
    false,
  );
  assert.doesNotMatch(result.stdout + result.stderr, /secret-sentinel/);
});
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
