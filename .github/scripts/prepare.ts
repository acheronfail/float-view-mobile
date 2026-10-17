import { appendFileSync, writeFileSync } from 'node:fs';
import { ciBuildIdentity } from './build-record.ts';
import { getNightlyRelease, getRelease, sourceMarker } from './github.ts';
import { assetNames, git, isAncestor, resolveVersion } from './version.ts';

const commit = git(['rev-parse', 'HEAD']);
if (commit !== process.env.GITHUB_SHA) throw new Error('Checkout differs from workflow source');
const ref = process.env.GITHUB_REF ?? '';
const isTag = ref.startsWith('refs/tags/');
if (!isTag && ref !== 'refs/heads/master') throw new Error('Nightly builds require master');
const metadata = resolveVersion(commit, isTag ? ref.slice('refs/tags/'.length) : null, process.cwd(), 'v1.0.0');
const event = process.env.GITHUB_EVENT_NAME;
const delivery = process.env.FLOAT_VIEW_IOS_DELIVERY || 'none';
if (!['none', 'check', 'upload'].includes(delivery)) throw new Error('Invalid iOS delivery action');
const explicitlyRequested = process.env.FLOAT_VIEW_BUILD_IOS_STORE === 'true' || delivery !== 'none';
if (explicitlyRequested && (!isTag || event !== 'workflow_dispatch'))
  throw new Error('Manual store preparation requires a stable release tag or numbered candidate; never master or nightly');
const storeRequested = isTag && (event === 'push' || (event === 'workflow_dispatch' && explicitlyRequested));
const storeDelivery = storeRequested && event === 'push' ? 'upload' : delivery;
const macExplicitlyRequested = process.env.FLOAT_VIEW_BUILD_MAC_STORE === 'true';
if (macExplicitlyRequested && (!isTag || event !== 'workflow_dispatch'))
  throw new Error('Manual Mac store preparation requires a stable release tag or numbered candidate; never master or nightly');
const macStoreRequested = isTag && (event === 'push' || (event === 'workflow_dispatch' && macExplicitlyRequested));
const identity = ciBuildIdentity();
const existing = metadata.channel === 'nightly' ? getNightlyRelease() : getRelease(metadata.tag);
const publishedSource = existing?.body?.match(/<!-- float-view-source:([0-9a-f]{40}) -->/)?.[1];
const newerNightly = publishedSource && publishedSource !== commit && isAncestor(commit, publishedSource);
const publishedBuild = existing?.body?.match(/<!-- float-view-build-number:([1-9][0-9]*) -->/)?.[1];
const newerBuild = publishedBuild && Number(publishedBuild) > Number(identity.buildNumber);
// A stable tag added after publication may change the next nightly base for the same commit.
// Check the published version's asset set so an unchanged complete nightly still skips.
const publishedVersion = existing?.body?.match(/<!-- float-view-version:(v[^\s]+) -->/)?.[1];
const expectedAssets = assetNames(publishedVersion ?? metadata.version);
const completeAssets =
  existing?.assets.length === expectedAssets.length &&
  existing.assets.every((asset) => expectedAssets.includes(asset.name));
const skip =
  metadata.channel !== 'nightly'
    ? Boolean(existing && !existing.draft)
    : Boolean(
        existing &&
          !existing.draft &&
          (newerNightly || newerBuild || (completeAssets && existing.body?.includes(sourceMarker(commit))))
      );
const output = {
  ...metadata, storeBuild: String(storeRequested), storeDelivery, macStoreBuild: String(macStoreRequested),
  buildNumber: identity.buildNumber, runURL: identity.runURL,
  buildRecordArtifact: `build-record-${identity.attempt}`, commit, skip: String(skip),
};
writeFileSync(process.env.FLOAT_VIEW_BUILD_RECORD!, JSON.stringify({ schema: 1, ...identity, ...metadata, commit }, null, 2) + '\n');
appendFileSync(
  process.env.GITHUB_OUTPUT!,
  Object.entries(output)
    .map(([key, value]) => `${key}=${value}\n`)
    .join('')
);
console.log(JSON.stringify(output));
