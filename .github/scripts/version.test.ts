import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, test } from 'node:test';
import { buildNumber, bumpVersion, git, latestStable, parseVersion, resolveVersion } from './version.ts';
import { preview, release } from './release.ts';
import { releaseNotes } from './notes.ts';

const directories: string[] = [];
let commitNumber = 0;
afterEach(() => {
  for (const dir of directories.splice(0)) rmSync(dir, { recursive: true, force: true });
});
function repository() {
  const dir = mkdtempSync(join(tmpdir(), 'float-release-'));
  directories.push(dir);
  git(['init', '-b', 'master', dir]);
  git(['config', 'user.email', 'test@example.invalid'], dir);
  git(['config', 'user.name', 'Test'], dir);
  commit(dir);
  return dir;
}
function commit(dir: string) {
  git(['-c', 'core.hooksPath=/dev/null', 'commit', '--allow-empty', '-m', `test ${++commitNumber}`], dir);
  return git(['rev-parse', 'HEAD'], dir);
}
function remote(dir: string) {
  const origin = `${dir}-origin`;
  directories.push(origin);
  git(['init', '--bare', origin]);
  git(['remote', 'add', 'origin', origin], dir);
  git(['push', '-u', 'origin', 'master', '--tags'], dir);
  return origin;
}

test('semantic bumps reset lower components and parse both display channels', () => {
  assert.equal(bumpVersion('v0.1.49', 'patch'), 'v0.1.50');
  assert.equal(bumpVersion('v0.1.49', 'minor'), 'v0.2.0');
  assert.equal(bumpVersion('v0.1.49', 'major'), 'v1.0.0');
  assert.equal(parseVersion('v2.3.4-nightly.12.gabcdef012345').appVersion, '2.3.4');
  for (const bad of [
    'v01.2.3',
    '0.1.0',
    'v0.1',
    'v0.1.1-beta',
    'v1.2.3-nightly.01.gabcdef0',
    'v1.2.3\n',
    'v1.2.9007199254740992',
  ]) {
    assert.throws(() => parseVersion(bad));
  }
  assert.equal(buildNumber('60'), '60');
  for (const bad of [undefined, '0', '01', '-1', '2147483647']) assert.throws(() => buildNumber(bad));
});

test('greatest numeric stable ancestor determines nightly base, excluding unrelated tags', () => {
  const dir = repository();
  git(['tag', 'v0.1.9'], dir);
  git(['tag', 'v0.1.49'], dir);
  git(['tag', 'nightly'], dir);
  git(['checkout', '-b', 'unrelated'], dir);
  commit(dir);
  git(['tag', 'v9.0.0'], dir);
  git(['checkout', 'master'], dir);
  const sha = commit(dir);
  assert.equal(latestStable(sha, dir), 'v0.1.49');
  assert.equal(resolveVersion(sha, null, dir).version, `v0.1.50-nightly.1.g${sha.slice(0, 12)}`);
  git(['tag', 'v0.2.0'], dir);
  git(['tag', 'v0.3.0'], dir);
  assert.equal(latestStable(sha, dir, 'v0.2.0'), 'v0.1.49');
  git(['tag', '-d', 'v0.3.0'], dir);
  const later = commit(dir);
  assert.equal(resolveVersion(later, null, dir).version, `v0.2.1-nightly.1.g${later.slice(0, 12)}`);
});

test('tag builds require an exact stable tag and master ancestry', () => {
  const dir = repository();
  git(['tag', 'v0.2.0'], dir);
  remote(dir);
  const sha = git(['rev-parse', 'HEAD'], dir);
  assert.deepEqual(resolveVersion(sha, 'v0.2.0', dir), {
    version: 'v0.2.0',
    appVersion: '0.2.0',
    channel: 'stable',
    tag: 'v0.2.0',
    configuration: 'Release',
  });
  assert.throws(() => resolveVersion(sha, 'v0.2.0-nightly.0.gabcdef0', dir));
  const next = commit(dir);
  assert.throws(() => resolveVersion(next, 'v0.2.0', dir), /does not match/);
  git(['tag', 'v0.3.0'], dir);
  assert.throws(() => resolveVersion(next, 'v0.3.0', dir), /master history/);
});

test('release helper pushes an annotated semantic tag without modifying source', () => {
  const dir = repository();
  git(['tag', 'v0.1.49'], dir);
  const origin = remote(dir);
  assert.equal(release('minor', dir), 'v0.2.0');
  assert.equal(git(['cat-file', '-t', 'refs/tags/v0.2.0'], origin), 'tag');
  assert.equal(git(['rev-parse', 'v0.2.0^{commit}'], origin), git(['rev-parse', 'HEAD'], dir));
  assert.equal(git(['status', '--porcelain'], dir), '');
});

test('release helper rejects dirty, non-master and unpushed commits', () => {
  const dir = repository();
  remote(dir);
  writeFileSync(join(dir, 'untracked'), 'changed');
  assert.throws(() => release('patch', dir), /clean/);
  rmSync(join(dir, 'untracked'));
  git(['checkout', '-b', 'feature'], dir);
  assert.throws(() => release('patch', dir), /master branch/);
  git(['checkout', 'master'], dir);
  commit(dir);
  assert.throws(() => release('patch', dir), /match origin\/master/);
  assert.throws(() => release('wrong', dir), /Usage/);
  assert.equal(git(['tag', '--list'], dir), '');
});

test('native metadata strips nightly suffixes and keeps an independent build number', () => {
  const script = new URL('./version.ts', import.meta.url).pathname;
  const result = spawnSync(process.execPath, ['--strip-types', script, 'native'], {
    encoding: 'utf8',
    env: { ...process.env, FLOAT_VIEW_VERSION: 'v2.0.0-nightly.3.gabcdef012345', FLOAT_VIEW_BUILD_NUMBER: '600' },
  });
  assert.equal(result.status, 0);
  assert.equal(result.stdout.trim(), '2.0.0 600');
  const invalid = spawnSync(process.execPath, ['--strip-types', script, 'native'], {
    encoding: 'utf8',
    env: { ...process.env, FLOAT_VIEW_VERSION: 'v2.0.0', FLOAT_VIEW_BUILD_NUMBER: '' },
  });
  assert.notEqual(invalid.status, 0);
  assert.match(invalid.stderr, /FLOAT_VIEW_BUILD_NUMBER/);
});

test('a stale local rolling nightly tag does not block stable release creation', () => {
  const dir = repository();
  git(['tag', 'v0.1.49'], dir);
  git(['tag', 'nightly'], dir);
  const origin = remote(dir);
  const next = commit(dir);
  git(['push', 'origin', 'master'], dir);
  git(['update-ref', 'refs/tags/nightly', next], origin);
  assert.equal(release('patch', dir), 'v0.1.50');
  assert.equal(git(['rev-parse', 'v0.1.50^{commit}'], origin), next);
});

test('release preview uses publication notes on dirty, unpushed branches without a remote', () => {
  const dir = repository();
  git(['tag', 'v0.1.49'], dir);
  git(['checkout', '-b', 'feature'], dir);
  git(['-c', 'core.hooksPath=/dev/null', 'commit', '--allow-empty', '-m', 'app: Show speed'], dir);
  writeFileSync(join(dir, 'untracked'), 'not committed');
  const before = git(['status', '--porcelain'], dir);
  const result = preview('HEAD', dir);
  assert.equal(result.version, 'v0.1.50');
  assert.equal(result.previousTag, 'v0.1.49');
  assert.equal(
    result.notes,
    releaseNotes(
      result.version,
      result.commit,
      result.previousTag,
      process.env.GH_REPO ?? 'acheronfail/float-view-mobile',
      dir
    )
  );
  assert.match(result.notes, /## App\n\n- Show speed/);
  assert.doesNotMatch(result.notes, /not committed/);
  assert.equal(git(['tag', '--list'], dir), 'v0.1.49');
  assert.equal(git(['status', '--porcelain'], dir), before);
  const historical = preview('master', dir);
  assert.match(historical.notes, /No changes since the previous stable release/);
  assert.throws(() => preview('missing-ref', dir));
});

test('release preview CLI prints Markdown and supports the first release', () => {
  const dir = repository();
  const script = new URL('./release.ts', import.meta.url).pathname;
  const result = spawnSync(process.execPath, ['--strip-types', script, 'preview'], {
    cwd: dir,
    encoding: 'utf8',
    env: { ...process.env, GH_REPO: 'test/repo' },
  });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stderr, /Previewing v0\.0\.1: start of history\.\.HEAD/);
  assert.match(result.stdout, /^## Other changes/);
  assert.match(result.stdout, /https:\/\/github.com\/test\/repo\/commits\/v0.0.1/);
  assert.equal(git(['tag', '--list'], dir), '');
});

test('launch nightlies target v1.0.0 and keep their count through pre-v1 tag cleanup', () => {
  const dir = repository();
  git(['tag', 'v0.3.0'], dir);
  const sha = commit(dir);
  const before = resolveVersion(sha, null, dir, 'v1.0.0');
  assert.equal(before.version, `v1.0.0-nightly.2.g${sha.slice(0, 12)}`);
  git(['tag', '-d', 'v0.3.0'], dir);
  assert.equal(resolveVersion(sha, null, dir, 'v1.0.0').version, before.version);
  git(['tag', 'v1.0.0'], dir);
  const after = commit(dir);
  assert.equal(resolveVersion(after, null, dir, 'v1.0.0').version, `v1.0.1-nightly.1.g${after.slice(0, 12)}`);
});
