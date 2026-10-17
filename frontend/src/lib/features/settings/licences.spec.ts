import { describe, expect, it } from 'vitest';
import notices from '../../../../../THIRD_PARTY_NOTICES.txt?raw';
import { groupThirdPartyNotices } from './licences';

describe('third-party licence groups', () => {
  it('preserves every component and resolves every shared document offline', () => {
    const { groups, introduction } = groupThirdPartyNotices(notices);
    const entries = groups.flatMap((group) => group.entries);
    const inventory = notices.split('='.repeat(72))[0];
    const components = [...inventory.matchAll(/^(.+)\nLicence: /gm)].map((match) => match[1]);
    expect(entries.map((entry) => entry.title)).toEqual(components);
    expect(introduction).toContain('Dependency licences remain independent');
    const parts = notices.split(/^={72}\nDocument ([a-f0-9]{16})\n={72}\n/m);
    for (let index = 1; index < parts.length; index += 2) {
      const key = parts[index];
      const document = parts[index + 1].trim();
      const owners = inventory
        .trim()
        .split('\n\n')
        .filter((entry) => entry.includes(`document ${key}`));
      expect(owners.length, key).toBeGreaterThan(0);
      for (const owner of owners) {
        expect(entries.find((entry) => entry.title === owner.split('\n')[0])?.text).toContain(document);
      }
    }
    expect(entries.every((entry) => !/document [a-f0-9]{16}/.test(entry.text))).toBe(true);
  });
});
