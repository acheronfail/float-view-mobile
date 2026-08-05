import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import ListItem from './ListItem.svelte';

describe('ListItem', () => {
  it('renders navigation items as links with shared interaction states', () => {
    const { body } = render(ListItem, {
      props: {
        title: 'Trail board',
        subtitle: 'Configured',
        href: '#/boards/trail-board',
      },
    });

    expect(body).toContain('<a');
    expect(body).toContain('href="#/boards/trail-board"');
    expect(body).toContain('hover:bg-surface');
    expect(body).toContain('active:bg-accent-soft');
    expect(body).toContain('focus-visible:outline-accent');
  });

  it('renders action items as full-width disabled buttons', () => {
    const { body } = render(ListItem, {
      props: {
        title: 'Discovered board',
        subtitle: 'AA:BB:CC',
        onactivate: () => undefined,
        disabled: true,
      },
    });

    expect(body).toContain('<button');
    expect(body).toContain('type="button"');
    expect(body).toContain('disabled');
    expect(body).not.toContain('<a');
  });
});
