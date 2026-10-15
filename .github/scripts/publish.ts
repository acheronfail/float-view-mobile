import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { api, getRelease, sourceMarker, type Release } from './github.ts';
import { assetNames, git, parseVersion, resolveVersion } from './version.ts';

const commit = process.env.GITHUB_SHA!;
const version = process.env.FLOAT_VIEW_VERSION!;
parseVersion(version);
const stableTag = process.env.GITHUB_REF?.startsWith('refs/tags/') ? process.env.GITHUB_REF.slice(10) : null;
const channel = stableTag ? 'stable' : 'nightly';
const tag = stableTag ?? 'nightly';
if (stableTag) {
  if (resolveVersion(commit, stableTag).version !== version)
    throw new Error('Stable publication version differs from its tag');
} else if (!version.endsWith(`.g${git(['rev-parse', '--short=12', commit])}`)) {
  throw new Error('Nightly version does not identify the build commit');
}
// Use the prepared nightly base: a newly pushed stable tag must not invalidate an in-flight build.
const stable = channel === 'stable';
const files = assetNames(version);
for (const file of files) if (!existsSync(file)) throw new Error(`Missing release asset ${file}`);
let release = getRelease(tag);
if (stable && release && !release.draft) {
  console.log(`${tag} is already published; preserving its assets.`);
} else {
  const body = `Version ${version} from commit ${commit}.\n\nIPA for AltStore, signed APK for Android sideloading, and self-signed universal macOS app (macOS 14+, not notarized). These are not app-store submissions.\n\n${sourceMarker(commit)}`;
  if (!release) {
    release = api<Release>('releases', 'POST', {
      tag_name: tag,
      target_commitish: commit,
      name: stable ? version : 'Nightly',
      draft: true,
      prerelease: !stable,
      make_latest: 'false',
    });
  }
  if (!release) throw new Error('GitHub did not return a release');
  // Upload the replacement set first. Previous nightly assets survive any failed upload.
  execFileSync('gh', ['release', 'upload', tag, ...files, '--clobber'], { stdio: 'inherit' });
  if (!stable) {
    // A draft release may not have created its Git ref yet.
    const ref = api<{ object: { sha: string } }>('git/ref/tags/nightly', 'GET', undefined, true);
    if (ref) api('git/refs/tags/nightly', 'PATCH', { sha: commit, force: true });
    else api('git/refs', 'POST', { ref: 'refs/tags/nightly', sha: commit });
  } else if (git(['rev-parse', `${tag}^{commit}`]) !== commit) {
    throw new Error('Stable tag moved during publication');
  }
  api(`releases/${release.id}`, 'PATCH', {
    name: stable ? version : 'Nightly',
    body,
    draft: false,
    prerelease: !stable,
    make_latest: stable ? 'true' : 'false',
  });
  // Only after successful publication remove superseded nightly or interrupted draft assets.
  const current = getRelease(tag);
  if (!current) throw new Error('Published release is unavailable');
  for (const asset of current.assets) {
    if (!files.includes(asset.name)) api(`releases/assets/${asset.id}`, 'DELETE');
  }
  console.log(`Published ${version} (${channel}).`);
}
