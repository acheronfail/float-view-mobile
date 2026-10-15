import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import SegmentedControl from './SegmentedControl.svelte';

describe('segmented choices', () => {
  it('shows three real choices as pressed buttons, excluding the dropdown placeholder', () => {
    const { body } = render(SegmentedControl, {
      props: {
        label: 'Mode',
        options: [
          { value: '', label: 'Choose', disabled: true },
          ...['A', 'B', 'C'].map((value) => ({ value, label: value })),
        ],
        value: 'B',
        onchange() {},
      },
    });
    expect(body.match(/<button/g)).toHaveLength(3);
    expect(body.match(/aria-pressed="true"/g)).toHaveLength(1);
    expect(body).toContain('aria-label="Mode"');
    expect(body).not.toContain('<select');
  });

  it('keeps larger lists in a labelled native dropdown with groups', () => {
    const { body } = render(SegmentedControl, {
      props: {
        label: 'Profile',
        options: ['A', 'B', 'C', 'D'].map((value) => ({ value, label: value, group: 'Profiles' })),
        value: 'C',
        disabled: true,
        describedby: 'help',
        onchange() {},
      },
    });
    expect(body).toContain('<select');
    expect(body).toContain('<optgroup label="Profiles">');
    expect(body).toContain('aria-describedby="help"');
    expect(body).toContain('disabled');
    expect(body).not.toContain('<button');
  });

  it('keeps compact selectors native even with only two choices', () => {
    const { body } = render(SegmentedControl, {
      props: {
        label: 'Section',
        variant: 'compact',
        options: [
          { value: 'a', label: 'Ride' },
          { value: 'b', label: 'Other' },
        ],
        value: 'a',
        disabled: true,
        describedby: 'help',
        onchange() {},
      },
    });
    expect(body).toContain('<select');
    expect(body).not.toContain('<button');
    expect(body).toContain('aria-describedby="help"');
    expect(body).toContain('disabled');
  });
});
