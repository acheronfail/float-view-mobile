import { pathToFileURL } from 'node:url';
import { releaseNotes, releaseNotesBase } from './notes.ts';
import { bumpVersion, git, latestStable, parseVersion, type Bump } from './version.ts';

export function preview(ref = 'HEAD', cwd = process.cwd()) {
  const commit = git(['rev-parse', '--verify', '--end-of-options', `${ref}^{commit}`], cwd);
  const version = bumpVersion(latestStable(commit, cwd) ?? 'v0.0.0', 'patch');
  const previousTag = releaseNotesBase(version, commit, cwd);
  const notes = releaseNotes(version, commit, previousTag, process.env.GH_REPO ?? 'acheronfail/float-view-mobile', cwd);
  return { commit, previousTag, version, notes };
}

export function release(bump: string, cwd = process.cwd(), candidateBump?: string): string {
  if (!['patch', 'minor', 'major', 'candidate'].includes(bump))
    throw new Error('Usage: just release patch|minor|major|candidate');
  if (candidateBump !== undefined && (bump !== 'candidate' || !['patch', 'minor', 'major'].includes(candidateBump)))
    throw new Error('Usage: just release candidate [patch|minor|major]');
  if (git(['status', '--porcelain'], cwd)) throw new Error('Release requires a clean working tree');
  if (git(['branch', '--show-current'], cwd) !== 'master') throw new Error('Release requires the master branch');
  git(
    ['fetch', 'origin', '--no-tags', '+refs/heads/master:refs/remotes/origin/master', 'refs/tags/v*:refs/tags/v*'],
    cwd
  );
  const commit = git(['rev-parse', 'HEAD'], cwd);
  if (commit !== git(['rev-parse', 'origin/master'], cwd))
    throw new Error('Master must match origin/master; push or pull first');
  const base = latestStable(commit, cwd) ?? 'v0.0.0';
  let version: string;
  if (bump === 'candidate') {
    const target = candidateBump
      ? bumpVersion(base, candidateBump as Bump)
      : parseVersion(base).major < 1
        ? 'v1.0.0'
        : bumpVersion(base, 'patch');
    if (git(['tag', '--list', target], cwd)) throw new Error(`Stable version ${target} is already reserved`);
    const prefix = `${target}-rc.`;
    const numbers = git(['tag', '--list', `${prefix}*`], cwd)
      .split('\n')
      .map((tag) => tag.slice(prefix.length))
      .filter((value) => /^[1-9][0-9]*$/.test(value))
      .map(Number);
    version = `${prefix}${Math.max(0, ...numbers) + 1}`;
    parseVersion(version);
  } else version = bumpVersion(base, bump as Bump);
  if (git(['tag', '--list', version], cwd)) throw new Error(`Tag ${version} is already reserved`);
  git(['tag', '-a', version, '-m', `Float View ${version}`, commit], cwd);
  // Keep the local tag on a failed push so a retry cannot accidentally select another commit.
  git(['push', 'origin', `refs/tags/${version}:refs/tags/${version}`], cwd);
  return version;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const action = process.argv[2] ?? '';
  const ref = process.argv[3];
  if (process.argv.length > 4 || (!['preview', 'candidate'].includes(action) && ref))
    throw new Error(
      'Usage: just release patch|minor|major, just release candidate [patch|minor|major], or just release preview [ref]'
    );
  if (action === 'preview') {
    const result = preview(ref);
    console.error(
      `Previewing ${result.version}: ${result.previousTag ?? 'start of history'}..${ref ?? 'HEAD'} (${result.commit})`
    );
    console.log(result.notes);
  } else {
    console.log(`Pushed ${release(action, process.cwd(), ref)}; the tag starts the release workflow.`);
  }
}
