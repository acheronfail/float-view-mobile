import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type {
  RefloatConfigurationField,
  RefloatConfigurationIdentity,
  RefloatConfigurationSchemaPage,
  RefloatConfigurationValue,
  RefloatConfigurationValues,
} from '$lib/backend/contract';
import RefloatConfigurationFieldView from './RefloatConfigurationField.svelte';
import RefloatConfigurationEditor from './RefloatConfigurationEditor.svelte';
import {
  RefloatConfigurationController,
  formatRefloatValue,
  refloatConfigurationFieldHelp,
  refloatConfigurationFieldLabel,
  refloatConfigurationSectionKey,
  resolveRefloatConfigurationSection,
  validateRefloatField,
  type RefloatConfigurationBackend,
} from './refloat-configuration.svelte';

const identity: RefloatConfigurationIdentity = {
  packageName: 'refloat',
  packageVersion: '1.2.3',
  schemaFingerprint: 'a'.repeat(64),
  wireSignature: 123,
};

const field = (
  name: string,
  type: RefloatConfigurationField['type'],
  defaultValue: RefloatConfigurationValue,
  overrides: Partial<RefloatConfigurationField> = {}
): RefloatConfigurationField => ({
  name,
  label: name,
  type,
  defaultValue,
  minimum: null,
  maximum: null,
  step: null,
  precision: null,
  suffix: '',
  choices: [],
  description: `${name} help`,
  ...overrides,
});

const fields = [
  field('gain', 'double', { type: 'double', value: 1 }, { minimum: 0, maximum: 2, step: 0.1, suffix: 'A' }),
  field('count', 'integer', { type: 'integer', value: 2 }, { minimum: 0, maximum: 5 }),
  field('mode', 'enum', { type: 'integer', value: 0 }, { choices: ['Calm', 'Sport'] }),
  field('enabled', 'boolean', { type: 'boolean', value: true }),
  field('label', 'string', { type: 'string', value: 'board' }),
  field('flags', 'bitfield', { type: 'integer', value: 3 }),
  field('leds.on', 'boolean', { type: 'boolean', value: true }, { label: 'LEDs On' }),
];

const values = Object.fromEntries(fields.map((item) => [item.name, item.defaultValue]));

class FakeBackend implements RefloatConfigurationBackend {
  writes: Array<{
    identity: RefloatConfigurationIdentity;
    revision: number;
    values: Record<string, RefloatConfigurationValue>;
  }> = [];
  pages: RefloatConfigurationSchemaPage[] = [
    { identity, group: 'General', subgroup: 'Ride', fields: fields.slice(0, 4), nextCursor: 'page-2' },
    { identity, group: 'General', subgroup: 'Other', fields: fields.slice(4), nextCursor: null },
  ];
  async openRefloatConfiguration() {
    return {
      availability: 'ready' as const,
      writable: true,
      packageVersion: identity.packageVersion,
      schemaFingerprint: identity.schemaFingerprint,
      wireSignature: identity.wireSignature,
      currentLoaded: true,
      defaultLoaded: false,
      pendingOperation: null,
    };
  }
  async refloatConfigurationSchema(cursor: string | null = null) {
    return this.pages[cursor ? 1 : 0] ?? null;
  }
  async readRefloatConfiguration(source: 'current' | 'default') {
    return { identity, revision: source === 'current' ? 4 : 1, source, values } as RefloatConfigurationValues;
  }
  async writeRefloatConfiguration(
    requestIdentity: RefloatConfigurationIdentity,
    revision: number,
    requestValues: Record<string, RefloatConfigurationValue>
  ) {
    this.writes.push({ identity: requestIdentity, revision, values: requestValues });
    return { identity, revision: revision + 1, source: 'current' as const, values: requestValues };
  }
}

describe('generated Refloat configuration', () => {
  it('loads deterministic pages, tracks source and dirty state, and writes exactly once', async () => {
    const backend = new FakeBackend();
    const controller = new RefloatConfigurationController(backend);
    await controller.open();
    expect(controller.pages).toHaveLength(2);
    expect(controller.loaded?.revision).toBe(4);
    controller.setValue('gain', { type: 'double', value: 1.2 });
    expect(controller.diff.map((item) => item.name)).toEqual(['gain']);
    expect(controller.canWrite).toBe(true);
    expect(await controller.write()).toBe(true);
    expect(backend.writes).toHaveLength(1);
    expect(backend.writes[0]).toMatchObject({ identity, revision: 4 });
    expect(controller.diff).toHaveLength(0);
  });

  it('projects ordered logical sections, merges transport chunks, and qualifies duplicate subgroup labels', () => {
    const controller = new RefloatConfigurationController(new FakeBackend());
    controller.pages = [
      { identity, group: 'General', subgroup: 'Tune', fields: fields.slice(0, 2), nextCursor: 'page-2' },
      { identity, group: 'General', subgroup: 'Other', fields: fields.slice(4, 5), nextCursor: 'page-3' },
      { identity, group: 'General', subgroup: 'Tune', fields: fields.slice(2, 4), nextCursor: 'page-4' },
      { identity, group: 'Internal', subgroup: 'Other', fields: fields.slice(5), nextCursor: null },
    ];

    expect(controller.sections.map(({ key, label }) => ({ key, label }))).toEqual([
      { key: refloatConfigurationSectionKey('General', 'Tune'), label: 'Tune' },
      { key: refloatConfigurationSectionKey('General', 'Other'), label: 'General · Other' },
      { key: refloatConfigurationSectionKey('Internal', 'Other'), label: 'Internal · Other' },
    ]);
    expect(controller.sections[0]?.fields.map((item) => item.name)).toEqual(['gain', 'count', 'mode', 'enabled']);
    expect(controller.fields).toHaveLength(fields.length);
  });

  it('searches only the selected section without weakening complete-editor state', async () => {
    const controller = new RefloatConfigurationController(new FakeBackend());
    await controller.open();
    const ride = controller.sections[0]!;
    const other = controller.sections[1]!;

    controller.search = 'other';
    expect(controller.filteredSectionFields(ride)).toEqual([]);
    expect(controller.filteredSectionFields(other)).toEqual(other.fields);
    controller.search = 'enabled help';
    expect(controller.filteredSectionFields(ride).map((item) => item.name)).toEqual(['enabled']);
    controller.setValue('gain', { type: 'double', value: 1.2 });
    controller.setValue('label', { type: 'string', value: 'x'.repeat(4_097) });
    expect(controller.diff.map((item) => item.name)).toEqual(['gain', 'label']);
    expect(controller.validationErrors.label).toContain('long');
    expect(controller.canWrite).toBe(false);
  });

  it('resolves retained navigation identities and falls back when a schema replaces them', () => {
    const controller = new RefloatConfigurationController(new FakeBackend());
    controller.pages = new FakeBackend().pages;
    const sections = controller.sections;
    const selected = sections[1]!;

    expect(resolveRefloatConfigurationSection(sections, selected.key)).toBe(selected);
    expect(resolveRefloatConfigurationSection(sections, 'missing')?.key).toBe(sections[0]?.key);
    expect(resolveRefloatConfigurationSection([], selected.key)).toBeNull();
  });

  it('renders a native selector and exactly one selected logical section with scoped empty search results', async () => {
    const controller = new RefloatConfigurationController(new FakeBackend());
    await controller.open();
    const initial = render(RefloatConfigurationEditor, { props: { controller } }).body.replace(/\s+/g, ' ');

    expect(initial).toContain('<span class="font-medium">Section</span>');
    expect(initial).toContain('<option value="[&quot;General&quot;,&quot;Ride&quot;]" selected>Ride</option>');
    expect(initial).toContain('<option value="[&quot;General&quot;,&quot;Other&quot;]">Other</option>');
    expect(initial).toContain('General · Ride');
    expect(initial).toContain('Show help for gain');
    expect(initial).not.toContain('Show help for label');

    controller.search = 'not present anywhere';
    const empty = render(RefloatConfigurationEditor, { props: { controller } }).body;
    expect(empty).toContain('No fields match this search.');
    expect(empty).toContain('General · Ride');
    expect(empty).not.toContain('General · Other</h3>');
  });

  it('renders fields from every transport chunk of the selected logical section', () => {
    const controller = new RefloatConfigurationController(new FakeBackend());
    controller.pages = [
      { identity, group: 'General', subgroup: 'Tune', fields: fields.slice(0, 2), nextCursor: 'page-2' },
      { identity, group: 'General', subgroup: 'Other', fields: fields.slice(4), nextCursor: 'page-3' },
      { identity, group: 'General', subgroup: 'Tune', fields: fields.slice(2, 4), nextCursor: null },
    ];
    controller.draft = structuredClone(values);

    const body = render(RefloatConfigurationEditor, { props: { controller } }).body;
    expect(body.match(/>Tune<\/option>/g)).toHaveLength(1);
    expect(body).toContain('Show help for gain');
    expect(body).toContain('Show help for enabled');
    expect(body).not.toContain('Show help for label');
  });

  it('validates every generated field family and formats enum/suffix values', () => {
    expect(validateRefloatField(fields[0]!, { type: 'double', value: 3 })).toContain('Maximum');
    expect(validateRefloatField(fields[1]!, { type: 'integer', value: 1.5 })).toContain('whole');
    expect(validateRefloatField(fields[2]!, { type: 'integer', value: 4 })).toContain('listed');
    expect(validateRefloatField(fields[3]!, { type: 'integer', value: 1 })).toContain('on or off');
    expect(validateRefloatField(fields[4]!, { type: 'string', value: 'x'.repeat(4_097) })).toContain('long');
    expect(validateRefloatField(fields[5]!, { type: 'integer', value: 3 })).toBeNull();
    expect(formatRefloatValue({ type: 'integer', value: 1 }, fields[2])).toBe('Sport');
    expect(formatRefloatValue({ type: 'double', value: 1.2 }, fields[0])).toBe('1.2 A');
  });

  it('invalidates drafts on flywheel entry, reconnect, or exact identity replacement', async () => {
    const controller = new RefloatConfigurationController(new FakeBackend());
    await controller.open();
    controller.setValue('gain', { type: 'double', value: 1.2 });
    controller.reconcile({ ...controller.summary!, writable: false, currentLoaded: false });
    expect(controller.loaded).toBeNull();
    expect(controller.draft).toEqual({});
  });

  it('renders native inputs for all types and distinguishes persistent lighting defaults', () => {
    const markup = fields
      .map(
        (item) =>
          render(RefloatConfigurationFieldView, {
            props: { field: item, value: item.defaultValue, onchange: () => {}, onhelp: () => {} },
          }).body
      )
      .join('');
    expect(markup).toContain('type="number"');
    expect(markup).toContain('<select');
    expect(markup).toContain('type="checkbox"');
    expect(markup).toContain('type="text"');
    expect(markup).toContain('persistent startup default');
    expect(markup.match(/>\(\?\)<\/button>/g)).toHaveLength(fields.length);
    expect(markup).toContain('aria-label="Show help for gain"');
    expect(markup).not.toContain('gain help');
    expect(refloatConfigurationFieldLabel(fields.at(-1)!)).toBe('LEDs On (persistent startup default)');
    expect(refloatConfigurationFieldHelp({ ...fields[0]!, description: '' })).toBe('gain');
  });
});
