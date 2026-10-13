import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type {
  RefloatConfigurationField,
  RefloatEditorDraft,
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
  tunable: true,
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
  saved: RefloatEditorDraft | null = null;
  current = structuredClone(values);
  reads: Array<{ source: 'current' | 'default'; refresh: boolean }> = [];
  failSave = false;
  failRestore = false;
  async getRefloatEditorDraft() {
    if (this.failRestore) throw new Error('unavailable');
    return structuredClone(this.saved);
  }
  async setRefloatEditorDraft(_deviceId: string, _revision: number, draft: RefloatEditorDraft) {
    if (this.failSave) throw new Error('unavailable');
    this.saved = structuredClone(draft);
  }
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
  async readRefloatConfiguration(source: 'current' | 'default', refresh = true) {
    this.reads.push({ source, refresh });
    return {
      identity,
      revision: source === 'current' ? 4 : 1,
      source,
      values: source === 'current' ? this.current : values,
    } as RefloatConfigurationValues;
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

  it('starts collapsed with a quick selector and retains full controls and scoped search', async () => {
    const controller = new RefloatConfigurationController(new FakeBackend());
    await controller.open();
    const initial = render(RefloatConfigurationEditor, { props: { controller } }).body.replace(/\s+/g, ' ');

    expect(controller.position.controlsExpanded).toBe(false);
    expect(initial).toContain('aria-expanded="false"');
    expect(initial).toContain('<select aria-label="Section"');
    expect(initial).toContain('>Ride</option>');
    expect(initial).toContain('>Other</option>');
    expect(initial).not.toContain('Search this section');

    controller.setPosition({ controlsExpanded: true });
    const expanded = render(RefloatConfigurationEditor, { props: { controller } }).body.replace(/\s+/g, ' ');
    expect(expanded).toContain('<span class="font-medium">Section</span>');
    expect(expanded).toMatch(/aria-pressed="true"[^>]*>Ride<\/button>/);
    expect(expanded).toMatch(/aria-pressed="false"[^>]*>Other<\/button>/);
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

  it('preserves drafts but disables writing while current configuration is unavailable', async () => {
    const controller = new RefloatConfigurationController(new FakeBackend());
    await controller.open();
    controller.setValue('gain', { type: 'double', value: 1.2 });
    controller.reconcile({ ...controller.summary!, writable: false, currentLoaded: false });
    expect(controller.loaded).not.toBeNull();
    expect(controller.draft.gain).toEqual({ type: 'double', value: 1.2 });
    expect(controller.canWrite).toBe(false);
  });

  it('restores edits, unfinished inputs and editor position in a new controller without writing', async () => {
    const backend = new FakeBackend();
    const first = new RefloatConfigurationController(backend, 'board');
    await first.open();
    first.setValue('enabled', { type: 'boolean', value: false });
    first.setNumericInput('gain', '');
    first.setPosition({
      sectionKey: refloatConfigurationSectionKey('General', 'Other'),
      search: 'label',
      controlsExpanded: false,
      scrollTop: 123,
    });
    await first.flush();
    const second = new RefloatConfigurationController(backend, 'board');
    await second.open();
    expect(second.draft.enabled).toEqual({ type: 'boolean', value: false });
    expect(second.inputs.gain).toBe('');
    expect(second.validationErrors.gain).toContain('finite');
    expect(second.position).toEqual(first.position);
    expect(second.restored).toBe(true);
    expect(second.canWrite).toBe(false);
    expect(backend.writes).toEqual([]);
  });

  it('reuses retained values across repeated editor recreation while explicit reads refresh', async () => {
    const backend = new FakeBackend();
    for (let i = 0; i < 3; i++) {
      const controller = new RefloatConfigurationController(backend, 'board');
      await controller.open();
      await controller.flush();
      expect(controller.loaded?.revision).toBe(4);
    }
    expect(backend.reads).toEqual(Array.from({ length: 3 }, () => ({ source: 'current', refresh: false })));
    const controller = new RefloatConfigurationController(backend, 'board');
    await controller.open();
    await controller.load('current');
    await controller.load('default');
    expect(backend.reads.slice(-2)).toEqual([
      { source: 'current', refresh: true },
      { source: 'default', refresh: true },
    ]);
    await controller.flush();
    const restoredDefaults = new RefloatConfigurationController(backend, 'board');
    await restoredDefaults.open();
    expect(backend.reads.at(-1)).toEqual({ source: 'default', refresh: false });
  });

  it('read current and defaults replace the durable draft and unfinished input', async () => {
    const backend = new FakeBackend();
    const controller = new RefloatConfigurationController(backend, 'board');
    await controller.open();
    for (const source of ['current', 'default'] as const) {
      controller.setNumericInput('gain', '1.2');
      await controller.flush();
      await controller.load(source);
      await controller.flush();
      expect(backend.saved?.values).toEqual(values);
      expect(backend.saved?.inputs).toEqual({});
      expect(backend.saved?.source).toBe(source);
      expect(controller.inputs).toEqual({});
    }
  });

  it('keeps the complete form when the board changes and counts differences against the fresh read', async () => {
    const backend = new FakeBackend();
    const first = new RefloatConfigurationController(backend, 'board');
    await first.open();
    first.setValue('gain', { type: 'double', value: 1.2 });
    await first.flush();
    backend.current.gain = { type: 'double', value: 1.4 };
    backend.current.label = { type: 'string', value: 'updated elsewhere' };
    const second = new RefloatConfigurationController(backend, 'board');
    await second.open();
    expect(second.draft.label).toEqual(values.label);
    expect(second.draft.gain).toEqual({ type: 'double', value: 1.2 });
    expect(second.diff.map((d) => d.name)).toEqual(['gain', 'label']);
    expect(second.canWrite).toBe(true);
    await second.load('current');
    expect(second.draft).toEqual(backend.current);
    expect(second.diff).toEqual([]);
  });

  it('retains incompatible drafts for inspection until an explicit read replaces them', async () => {
    const backend = new FakeBackend();
    const first = new RefloatConfigurationController(backend, 'board');
    await first.open();
    first.setValue('gain', { type: 'double', value: 1.2 });
    await first.flush();
    backend.saved!.identity.packageVersion = 'other';
    const controller = new RefloatConfigurationController(backend, 'board');
    await controller.open();
    expect(controller.incompatible?.values.gain).toBeDefined();
    expect(controller.canWrite).toBe(false);
    controller.setPosition({ search: 'gain' });
    await controller.flush();
    expect(backend.saved!.identity.packageVersion).toBe('other');
    await controller.load('current');
    await controller.flush();
    expect(controller.incompatible).toBeNull();
    expect(backend.saved!.identity).toEqual(identity);
  });

  it('coalesces rapid edits and preserves an undo while a newer save is in flight', async () => {
    const backend = new FakeBackend();
    const controller = new RefloatConfigurationController(backend, 'board');
    await controller.open();
    await controller.flush();
    let release: (() => void) | undefined;
    const save = backend.setRefloatEditorDraft.bind(backend);
    let calls = 0;
    backend.setRefloatEditorDraft = async (device, revision, draft) => {
      calls++;
      if (calls === 1)
        await new Promise<void>((resolve) => {
          release = resolve;
        });
      await save(device, revision, draft);
    };
    controller.setValue('gain', { type: 'double', value: 1.2 });
    controller.setValue('gain', { type: 'double', value: 1.4 });
    controller.setValue('gain', values.gain!);
    release!();
    await controller.flush();
    expect(backend.saved?.values.gain).toEqual(values.gain);
    expect(calls).toBe(2);
  });

  it('surfaces save failures, retries and never overwrites a draft after a failed restore', async () => {
    const backend = new FakeBackend();
    const controller = new RefloatConfigurationController(backend, 'board');
    await controller.open();
    await controller.flush();
    backend.failSave = true;
    controller.setValue('gain', { type: 'double', value: 1.2 });
    expect(await controller.flush()).toBe(false);
    expect(controller.saveError).toBeTruthy();
    backend.failSave = false;
    expect(await controller.flush()).toBe(true);
    const saved = structuredClone(backend.saved);
    backend.failRestore = true;
    const second = new RefloatConfigurationController(backend, 'board');
    await second.open();
    second.setPosition({ search: 'new' });
    expect(await second.flush()).toBe(false);
    expect(backend.saved).toEqual(saved);
  });

  it('renders accessible controls for all types and distinguishes persistent lighting defaults', () => {
    const markup = fields
      .map(
        (item) =>
          render(RefloatConfigurationFieldView, {
            props: { field: item, value: item.defaultValue, onchange: () => {}, onhelp: () => {} },
          }).body
      )
      .join('');
    expect(markup).toContain('type="number"');
    expect(markup).toContain('aria-label="mode"');
    expect(markup).toContain('aria-pressed="true"');
    expect(markup).not.toContain('type="checkbox"');
    expect(markup).toContain('type="text"');
    expect(markup).toContain('persistent startup default');
    expect(markup.match(/>\(\?\)<\/button>/g)).toHaveLength(fields.length);
    expect(markup).toContain('aria-label="Show help for gain"');
    expect(markup).not.toContain('gain help');
    expect(refloatConfigurationFieldLabel(fields.at(-1)!)).toBe('LEDs On (persistent startup default)');
    expect(refloatConfigurationFieldHelp({ ...fields[0]!, description: '' })).toBe('gain');
  });
});
