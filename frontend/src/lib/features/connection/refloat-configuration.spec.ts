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
import {
  RefloatConfigurationController,
  formatRefloatValue,
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
  it('loads deterministic pages, tracks source and dirty state, searches, and writes exactly once', async () => {
    const backend = new FakeBackend();
    const controller = new RefloatConfigurationController(backend);
    await controller.open();
    expect(controller.pages).toHaveLength(2);
    expect(controller.loaded?.revision).toBe(4);
    controller.search = 'other';
    expect(controller.filteredPages).toHaveLength(1);
    controller.search = '';
    controller.setValue('gain', { type: 'double', value: 1.2 });
    expect(controller.diff.map((item) => item.name)).toEqual(['gain']);
    expect(controller.canWrite).toBe(true);
    expect(await controller.write()).toBe(true);
    expect(backend.writes).toHaveLength(1);
    expect(backend.writes[0]).toMatchObject({ identity, revision: 4 });
    expect(controller.diff).toHaveLength(0);
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
            props: { field: item, value: item.defaultValue, onchange: () => {} },
          }).body
      )
      .join('');
    expect(markup).toContain('type="number"');
    expect(markup).toContain('<select');
    expect(markup).toContain('type="checkbox"');
    expect(markup).toContain('type="text"');
    expect(markup).toContain('persistent startup default');
  });
});
