import { pathToFileURL } from 'node:url';
import { bumpVersion, git, latestStable, type Bump } from './version.ts';

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
  console.log(`Pushed ${release(process.argv[2] ?? '')}; the tag starts the stable release workflow.`);
}
