import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, test } from 'node:test';
import { releaseNotes } from './notes.ts';
import { git } from './version.ts';

const directories: string[] = [];
afterEach(() => {
  for (const cwd of directories.splice(0)) rmSync(cwd, { recursive: true, force: true });
});

function fixture() {
  const cwd = mkdtempSync(join(tmpdir(), 'float-notes-'));
  directories.push(cwd);
  git(['init', '-b', 'master', cwd]);
  git(['config', 'user.email', 'test@example.invalid'], cwd);
  git(['config', 'user.name', 'Test'], cwd);
  function commit(title: string) {
    git(['-c', 'core.hooksPath=/dev/null', 'commit', '--allow-empty', '-m', title], cwd);
    return git(['rev-parse', 'HEAD'], cwd);
  }
  commit('app: Previous release');
  git(['tag', '-a', 'v0.1.0', '-m', 'Base release'], cwd);
  return {
    cwd,
    commit,
    notes: (base: string | null = 'v0.1.0') => releaseNotes('v0.2.0', 'HEAD', base, 'test/repo', cwd),
  };
}

test('direct commits are grouped by area with commit links and a stable comparison', () => {
  const f = fixture();
  f.commit('repo: Fix release counter');
  const app = f.commit('app: Show speed');
  f.commit('apple: Improve iOS voices');
  f.commit('android: Fix permissions');
  f.commit('windows: Support Windows Bluetooth');
  f.commit('linux: Integrate Linux permissions');
  f.commit('app: Record rides');
  const notes = f.notes();
  for (const heading of ['App', 'Apple', 'Android', 'Windows', 'Linux', 'Repository'])
    assert.ok(notes.includes(`## ${heading}\n`));
  assert.match(notes, /## Windows\n\n- Support Windows Bluetooth/);
  assert.match(notes, /## Linux\n\n- Integrate Linux permissions/);
  assert.ok(notes.includes(`- Show speed ([${app.slice(0, 7)}](https://github.com/test/repo/commit/${app}))`));
  assert.ok(notes.indexOf('Show speed') < notes.indexOf('Record rides'));
  assert.ok(notes.indexOf('## App\n') < notes.indexOf('## Repository\n'));
  assert.ok(notes.includes('https://github.com/test/repo/compare/v0.1.0...v0.2.0'));
  assert.doesNotMatch(notes, /Previous release|app:|## Other changes/);
});

test('unknown titles remain visible and Markdown in titles is escaped', () => {
  const f = fixture();
  f.commit('Fix an old issue');
  f.commit('build: Update tooling');
  f.commit('app: Show [speed](https://example.com) <details>');
  const notes = f.notes();
  assert.match(notes, /## Other changes\n\n- Fix an old issue/);
  assert.ok(notes.includes('build: Update tooling'));
  assert.ok(notes.includes('Show \\[speed\\](https://example.com) \\<details\\>'));
});

test('first releases include history and empty ranges have explicit notes', () => {
  const f = fixture();
  assert.match(f.notes(null), /Previous release/);
  assert.ok(f.notes(null).includes('https://github.com/test/repo/commits/v0.2.0'));
  assert.match(f.notes(), /No changes since the previous stable release/);
  assert.doesNotMatch(f.notes(), /## (App|Windows|Linux)/);
});

test('branch commits are included once while merge titles are omitted', () => {
  const f = fixture();
  git(['checkout', '-b', 'feature'], f.cwd);
  f.commit('app: Add board profile');
  git(['checkout', 'master'], f.cwd);
  git(['-c', 'core.hooksPath=/dev/null', 'merge', '--no-ff', 'feature', '-m', 'Merge feature'], f.cwd);
  const notes = f.notes();
  assert.equal(notes.match(/Add board profile/g)?.length, 1);
  assert.doesNotMatch(notes, /Merge feature/);
});
