import { appendFileSync } from 'node:fs';
import { getNightlyRelease, getRelease, sourceMarker } from './github.ts';
import { assetNames, buildNumber, git, isAncestor, resolveVersion } from './version.ts';

const commit = git(['rev-parse', 'HEAD']);
if (commit !== process.env.GITHUB_SHA) throw new Error('Checkout differs from workflow source');
const ref = process.env.GITHUB_REF ?? '';
const isTag = ref.startsWith('refs/tags/');
if (!isTag && ref !== 'refs/heads/master') throw new Error('Nightly builds require master');
const metadata = resolveVersion(commit, isTag ? ref.slice('refs/tags/'.length) : null);
const existing = metadata.channel === 'stable' ? getRelease(metadata.tag) : getNightlyRelease();
const publishedSource = existing?.body?.match(/<!-- float-view-source:([0-9a-f]{40}) -->/)?.[1];
const newerNightly = publishedSource && publishedSource !== commit && isAncestor(commit, publishedSource);
// A stable tag added after publication may change the next nightly base for the same commit.
// Check the published version's asset set so an unchanged complete nightly still skips.
const publishedVersion = existing?.body?.match(/<!-- float-view-version:(v[^\s]+) -->/)?.[1];
const expectedAssets = assetNames(publishedVersion ?? metadata.version);
const completeAssets =
  existing?.assets.length === expectedAssets.length &&
  existing.assets.every((asset) => expectedAssets.includes(asset.name));
const skip =
  metadata.channel === 'stable'
    ? Boolean(existing && !existing.draft)
    : Boolean(
        existing &&
          !existing.draft &&
          (newerNightly || (completeAssets && existing.body?.includes(sourceMarker(commit))))
      );
// ipa.yml ended at 60. Keep this offset and the current workflow path stable:
// renaming a workflow file starts a new GitHub run-number sequence.
const number = buildNumber(String(Number(buildNumber(process.env.GITHUB_RUN_NUMBER)) + 60));
const output = { ...metadata, buildNumber: number, commit, skip: String(skip) };
appendFileSync(
  process.env.GITHUB_OUTPUT!,
  Object.entries(output)
    .map(([key, value]) => `${key}=${value}\n`)
    .join('')
);
console.log(JSON.stringify(output));
