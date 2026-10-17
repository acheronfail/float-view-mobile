/** Read the component index and deduplicated documents emitted by generate-license-notices.py. */
export function groupThirdPartyNotices(text: string) {
  const parts = text.split(/^={72}\nDocument ([a-f0-9]{16})\n={72}\n/m);
  const inventory = parts[0].trim().split('\n\n');
  const introduction = inventory.splice(0, 2).join('\n\n');
  const documents = new Map<string, string>();
  for (let i = 1; i < parts.length; i += 2) documents.set(parts[i], parts[i + 1].trim());
  const groups = new Map<string, { title: string; text: string }[]>();
  for (const entry of inventory) {
    const title = entry.split('\n')[0];
    const group = title.startsWith('Rust:')
      ? 'Rust dependencies'
      : title.startsWith('Frontend:')
        ? 'Frontend dependencies'
        : title.startsWith('Android:')
          ? 'Android dependencies'
          : 'Artwork and bundled runtimes';
    const contents = entry.replace(/^ {2}(.+): document ([a-f0-9]{16})$/gm, (_, label: string, key: string) => {
      const document = documents.get(key);
      if (!document) throw new Error(`Missing licence document ${key}`);
      return `\n${label}\n${document}`;
    });
    const entries = groups.get(group) ?? [];
    entries.push({ title, text: contents });
    groups.set(group, entries);
  }
  return { introduction, groups: [...groups].map(([title, entries]) => ({ title, entries })) };
}
