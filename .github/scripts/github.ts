import { spawnSync } from 'node:child_process';
import { isNightlyVersion } from './version.ts';

export interface Release {
  id: number;
  tag_name: string;
  name: string;
  prerelease: boolean;
  draft: boolean;
  body: string | null;
  target_commitish?: string;
  assets: { id: number; name: string }[];
}

export function api<T>(path: string, method = 'GET', body?: object, allowMissing = false): T | null {
  const result = spawnSync(
    'gh',
    ['api', `repos/${process.env.GH_REPO ?? ''}/${path}`, '--method', method, ...(body ? ['--input', '-'] : [])],
    {
      encoding: 'utf8',
      input: body ? JSON.stringify(body) : undefined,
    }
  );
  if (result.error) throw result.error;
  if (result.status !== 0) {
    if (allowMissing && /\(HTTP 404\)/.test(result.stderr)) return null;
    throw new Error(`GitHub ${method} ${path} failed: ${result.stderr.trim()}`);
  }
  return (result.stdout.trim() ? JSON.parse(result.stdout) : null) as T | null;
}

export function getRelease(tag: string): Release | null {
  const published = api<Release>(`releases/tags/${encodeURIComponent(tag)}`, 'GET', undefined, true);
  if (published) return published;
  // The tag endpoint exposes published releases; list releases to recover an interrupted draft.
  return findRelease((release) => release.tag_name === tag);
}

export function getNightlyRelease(): Release | null {
  return findRelease((release) => release.name === 'Nightly' && release.prerelease);
}

export function pruneNightlyTags(currentTag: string): void {
  // Gather every page before deleting: removing tags changes pagination offsets.
  const obsolete: string[] = [];
  for (let page = 1; page <= 10; page++) {
    const tags = api<{ name: string }[]>(`tags?per_page=100&page=${page}`);
    if (!tags) throw new Error('GitHub did not return a tag list');
    obsolete.push(...tags.map((tag) => tag.name).filter((tag) => tag !== currentTag && isNightlyVersion(tag)));
    if (tags.length < 100) {
      for (const tag of obsolete) api(`git/refs/tags/${encodeURIComponent(tag)}`, 'DELETE');
      return;
    }
  }
  throw new Error('Tag search exceeded 1000 entries; refusing incomplete nightly cleanup');
}

function findRelease(matches: (release: Release) => boolean): Release | null {
  for (let page = 1; page <= 10; page++) {
    const releases = api<Release[]>(`releases?per_page=100&page=${page}`);
    if (!releases) throw new Error('GitHub did not return a release list');
    const release = releases.find(matches);
    if (release) return release;
    if (releases.length < 100) return null;
  }
  throw new Error('Release search exceeded 1000 entries; refusing to create a possible duplicate');
}

export function sourceMarker(commit: string): string {
  return `<!-- float-view-source:${commit} -->`;
}

export function versionMarker(version: string): string {
  return `<!-- float-view-version:${version} -->`;
}
