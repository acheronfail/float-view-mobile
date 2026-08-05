import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import PageHeader from './PageHeader.svelte';

describe('page header', () => {
  it('shows its bottom border by default', () => {
    const { body } = render(PageHeader, { props: { headingId: 'title', title: 'Title' } });

    expect(body).toContain('border-b');
    expect(body).toContain('border-b-accent-soft');
  });

  it('can hide its bottom border', () => {
    const { body } = render(PageHeader, {
      props: { headingId: 'title', title: 'Title', bottomBorder: false },
    });

    expect(body).not.toContain('border-b');
    expect(body).not.toContain('border-b-accent-soft');
  });
});
