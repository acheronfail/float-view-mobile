import { describe, expect, it } from 'vitest';
import { render } from 'svelte/server';
import type { RefloatConfigurationField } from '$lib/backend/contract';
import RefloatConfigurationDiff from './RefloatConfigurationDiff.svelte';

const field: RefloatConfigurationField = {
  name: 'gain',
  label: 'Gain',
  heading: null,
  type: 'double',
  defaultValue: { type: 'double', value: 1 },
  minimum: 0,
  maximum: 10,
  step: 0.1,
  precision: 1,
  suffix: 'A',
  choices: [],
  description: '',
  tunable: true,
};

describe('Refloat configuration differences', () => {
  it('renders compact labelled before-to-after rows with units', () => {
    const { body } = render(RefloatConfigurationDiff, {
      props: {
        changes: [
          { name: 'gain', label: 'Gain', before: { type: 'double', value: 1 }, after: { type: 'double', value: 4 } },
        ],
        fields: [field],
        beforeLabel: 'Current board',
        afterLabel: 'Saved tune',
      },
    });
    expect(body).toMatch(/1\s+setting will change/);
    expect(body).toContain('Current board → Saved tune');
    expect(body).toMatch(/<strong>Gain:<\/strong>\s*1 A → 4 A/);
    expect(body).not.toContain('<dl');
  });

  it('formats boolean and enum values and identifies missing values', () => {
    const { body } = render(RefloatConfigurationDiff, {
      props: {
        changes: [
          {
            name: 'enabled',
            label: 'Enabled',
            before: { type: 'boolean', value: false },
            after: { type: 'boolean', value: true },
          },
          { name: 'mode', label: 'Mode', before: undefined, after: { type: 'integer', value: 1 } },
        ],
        fields: [{ ...field, name: 'mode', label: 'Mode', type: 'enum', choices: ['Soft', 'Firm'] }],
        beforeLabel: 'Saved',
        afterLabel: 'Current board',
      },
    });
    expect(body).toContain('Off → On');
    expect(body).toContain('Not available → Firm');
  });

  it('shows contextual empty-state copy without an empty list', () => {
    const { body } = render(RefloatConfigurationDiff, {
      props: {
        changes: [],
        fields: [],
        beforeLabel: 'Current',
        afterLabel: 'New',
        emptyMessage: 'No differences. This tune already matches.',
      },
    });
    expect(body).toContain('No differences. This tune already matches.');
    expect(body).not.toContain('<ul');
  });
});
