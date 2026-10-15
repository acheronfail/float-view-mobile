import { git } from './version.ts';

const areas = [
  ['app', 'App'],
  ['apple', 'Apple'],
  ['android', 'Android'],
  ['windows', 'Windows'],
  ['linux', 'Linux'],
  ['repo', 'Repository'],
  ['other', 'Other changes'],
] as const;

function escapeMarkdown(value: string): string {
  return value.replace(/[\\`*_{}\[\]<>]/g, '\\$&');
}

export function releaseNotes(
  tag: string,
  commit: string,
  previousTag: string | null,
  repository: string,
  cwd = process.cwd()
): string {
  const url = `https://github.com/${repository}`;
  const range = previousTag ? `${previousTag}..${commit}` : commit;
  // Include direct and branch commits, without duplicating merge commit titles.
  const log = git(['log', '--reverse', '--no-merges', '--format=%H%x09%s', range, '--'], cwd);
  const groups = new Map<string, string[]>();
  for (const line of log ? log.split('\n') : []) {
    const separator = line.indexOf('\t');
    const sha = line.slice(0, separator);
    const title = line.slice(separator + 1);
    const match = /^([a-z]+): (.+)$/.exec(title);
    const area = areas.some(([prefix]) => prefix !== 'other' && prefix === match?.[1]) ? match![1]! : 'other';
    const entries = groups.get(area) ?? [];
    entries.push(
      `- ${escapeMarkdown(area === 'other' ? title : match![2]!)} ([${sha.slice(0, 7)}](${url}/commit/${sha}))`
    );
    groups.set(area, entries);
  }
  const sections = areas.flatMap(([area, heading]) => {
    const entries = groups.get(area);
    return entries ? [`## ${heading}\n\n${entries.join('\n')}`] : [];
  });
  if (!sections.length) sections.push('No changes since the previous stable release.');
  const changelog = previousTag
    ? `${url}/compare/${encodeURIComponent(previousTag)}...${encodeURIComponent(tag)}`
    : `${url}/commits/${encodeURIComponent(tag)}`;
  return `${sections.join('\n\n')}\n\n**Full Changelog**: ${changelog}`;
}
