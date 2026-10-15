import { pathToFileURL } from 'node:url';
import { releaseNotes } from './notes.ts';
import { bumpVersion, git, latestStable, type Bump } from './version.ts';

export function preview(ref = 'HEAD', cwd = process.cwd()) {
  const commit = git(['rev-parse', '--verify', '--end-of-options', `${ref}^{commit}`], cwd);
  const previousTag = latestStable(commit, cwd);
  const version = bumpVersion(previousTag ?? 'v0.0.0', 'patch');
  const notes = releaseNotes(version, commit, previousTag, process.env.GH_REPO ?? 'acheronfail/float-view-mobile', cwd);
  return { commit, previousTag, version, notes };
}

export function release(bump: string, cwd = process.cwd()): string {
  if (!['patch', 'minor', 'major'].includes(bump)) throw new Error('Usage: just release patch|minor|major');
  if (git(['status', '--porcelain'], cwd)) throw new Error('Release requires a clean working tree');
  if (git(['branch', '--show-current'], cwd) !== 'master') throw new Error('Release requires the master branch');
  git(
    ['fetch', 'origin', '--no-tags', '+refs/heads/master:refs/remotes/origin/master', 'refs/tags/v*:refs/tags/v*'],
    cwd
  );
  const commit = git(['rev-parse', 'HEAD'], cwd);
  if (commit !== git(['rev-parse', 'origin/master'], cwd))
    throw new Error('Master must match origin/master; push or pull first');
  const version = bumpVersion(latestStable(commit, cwd) ?? 'v0.0.0', bump as Bump);
  if (git(['tag', '--list', version], cwd)) throw new Error(`Tag ${version} is already reserved`);
  git(['tag', '-a', version, '-m', `Float View ${version}`, commit], cwd);
  // Keep the local tag on a failed push so a retry cannot accidentally select another commit.
  git(['push', 'origin', `refs/tags/${version}:refs/tags/${version}`], cwd);
  return version;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const action = process.argv[2] ?? '';
  const ref = process.argv[3];
  if (process.argv.length > 4 || (action !== 'preview' && ref))
    throw new Error('Usage: just release patch|minor|major or just release preview [ref]');
  if (action === 'preview') {
    const result = preview(ref);
    console.error(
      `Previewing ${result.version}: ${result.previousTag ?? 'start of history'}..${ref ?? 'HEAD'} (${result.commit})`
    );
    console.log(result.notes);
  } else {
    console.log(`Pushed ${release(action)}; the tag starts the stable release workflow.`);
  }
}
