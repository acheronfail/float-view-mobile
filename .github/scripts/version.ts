import { execFileSync, spawnSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

const integer = '(0|[1-9][0-9]*)';
const stablePattern = new RegExp(`^v${integer}\\.${integer}\\.${integer}$`);
const displayPattern = new RegExp(
  `^v${integer}\\.${integer}\\.${integer}(?:-nightly\\.${integer}\\.g[0-9a-f]{7,40})?$`
);
export type Bump = 'patch' | 'minor' | 'major';

export function parseVersion(value: string) {
  const match = displayPattern.exec(value);
  if (!match || match[0] !== value) throw new Error(`Invalid app version: ${value}`);
  const parts = match.slice(1, 4).map(Number);
  if (!parts.every(Number.isSafeInteger)) throw new Error('Version components exceed safe integers');
  return { major: parts[0]!, minor: parts[1]!, patch: parts[2]!, appVersion: parts.join('.') };
}

export function bumpVersion(value: string, bump: Bump): string {
  if (!stablePattern.test(value)) throw new Error('A stable base version is required');
  const { major, minor, patch } = parseVersion(value);
  const next =
    bump === 'major'
      ? `v${major + 1}.0.0`
      : bump === 'minor'
        ? `v${major}.${minor + 1}.0`
        : `v${major}.${minor}.${patch + 1}`;
  parseVersion(next);
  return next;
}

export function assetNames(version: string): string[] {
  parseVersion(version);
  return [`FloatView-${version}.ipa`, `FloatView-${version}.apk`, `FloatView-${version}-macos-universal.zip`];
}

export function buildNumber(value: string | undefined): string {
  if (!value || !/^[1-9][0-9]*$/.test(value) || Number(value) > 2_100_000_000) {
    throw new Error('FLOAT_VIEW_BUILD_NUMBER must be a positive integer no greater than 2100000000');
  }
  return value;
}

export function git(args: string[], cwd = process.cwd()): string {
  return execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
}

export function isAncestor(ancestor: string, descendant: string, cwd = process.cwd()): boolean {
  const result = spawnSync('git', ['merge-base', '--is-ancestor', ancestor, descendant], { cwd, encoding: 'utf8' });
  if (result.error) throw result.error;
  if (result.status !== 0 && result.status !== 1) throw new Error(result.stderr || 'Cannot resolve Git ancestry');
  return result.status === 0;
}

export function latestStable(commit: string, cwd = process.cwd()): string | null {
  const candidates = git(['tag', '--list'], cwd)
    .split('\n')
    .filter((tag) => stablePattern.test(tag));
  candidates.sort((a, b) => {
    const left = parseVersion(a),
      right = parseVersion(b);
    return right.major - left.major || right.minor - left.minor || right.patch - left.patch;
  });
  return candidates.find((tag) => isAncestor(tag, commit, cwd)) ?? null;
}

export function resolveVersion(commit: string, tag: string | null, cwd = process.cwd()) {
  if (tag !== null) {
    if (!stablePattern.test(tag)) throw new Error('Stable release tags must match vX.Y.Z exactly');
    parseVersion(tag);
    if (git(['rev-parse', `${tag}^{commit}`], cwd) !== commit)
      throw new Error('Release tag does not match the build commit');
    if (!isAncestor(commit, 'origin/master', cwd)) throw new Error('Release commit must be in origin/master history');
    return { version: tag, appVersion: parseVersion(tag).appVersion, channel: 'stable', tag, configuration: 'Release' };
  }
  const base = latestStable(commit, cwd);
  const count = git(['rev-list', '--count', ...(base ? [`${base}..${commit}`] : [commit])], cwd);
  const sha = git(['rev-parse', '--short=12', commit], cwd);
  const next = bumpVersion(base ?? 'v0.0.0', 'patch');
  const version = `${next}-nightly.${count}.g${sha}`;
  return {
    version,
    appVersion: parseVersion(version).appVersion,
    channel: 'nightly',
    tag: 'nightly',
    configuration: 'Debug',
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  if (process.argv[2] !== 'native') throw new Error('Usage: version.ts native');
  const version = parseVersion(process.env.FLOAT_VIEW_VERSION ?? '');
  console.log(`${version.appVersion} ${buildNumber(process.env.FLOAT_VIEW_BUILD_NUMBER)}`);
}
