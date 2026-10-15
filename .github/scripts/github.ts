import { spawnSync } from 'node:child_process';

export interface Release {
  id: number;
  tag_name: string;
  draft: boolean;
  body: string | null;
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
  for (let page = 1; page <= 10; page++) {
    const releases = api<Release[]>(`releases?per_page=100&page=${page}`);
    if (!releases) throw new Error('GitHub did not return a release list');
    const draft = releases.find((release) => release.tag_name === tag);
    if (draft) return draft;
    if (releases.length < 100) return null;
  }
  throw new Error('Release search exceeded 1000 entries; refusing to create a possible duplicate');
}

export function sourceMarker(commit: string): string {
  return `<!-- float-view-source:${commit} -->`;
}
