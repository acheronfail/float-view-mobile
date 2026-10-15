import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import {
  api,
  generatedNotes,
  getNightlyRelease,
  getRelease,
  sourceMarker,
  versionMarker,
  type Release,
} from './github.ts';
import { assetNames, git, latestStable, parseVersion, resolveVersion } from './version.ts';

const commit = process.env.GITHUB_SHA!;
const version = process.env.FLOAT_VIEW_VERSION!;
parseVersion(version);
const stableTag = process.env.GITHUB_REF?.startsWith('refs/tags/') ? process.env.GITHUB_REF.slice(10) : null;
const channel = stableTag ? 'stable' : 'nightly';
const tag = stableTag ?? version;
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
let release = stable ? getRelease(tag) : getNightlyRelease();
if (stable && release && !release.draft) {
  console.log(`${tag} is already published; preserving its assets.`);
} else {
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
  execFileSync('gh', ['release', 'upload', release.tag_name, ...files, '--clobber'], { stdio: 'inherit' });
  if (!stable) {
    // A SemVer tag lets GitHub order Nightly alongside stable releases correctly.
    // Versioned nightly tags identify immutable source snapshots; only the release/assets roll forward.
    const ref = api<{ object: { sha: string } }>(`git/ref/tags/${tag}`, 'GET', undefined, true);
    if (ref && ref.object.sha !== commit) throw new Error('Nightly version tag already points to another commit');
    if (!ref) api('git/refs', 'POST', { ref: `refs/tags/${tag}`, sha: commit });
  } else if (git(['rev-parse', `${tag}^{commit}`]) !== commit) {
    throw new Error('Stable tag moved during publication');
  }
  // Generate against the exact version tag, after ensuring it identifies this build.
  const previousTag = latestStable(commit, process.cwd(), stable ? tag : undefined);
  const notes = generatedNotes(tag, commit, previousTag);
  const body = `${notes}\n\n${sourceMarker(commit)}\n${versionMarker(version)}`;
  api(`releases/${release.id}`, 'PATCH', {
    tag_name: tag,
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
