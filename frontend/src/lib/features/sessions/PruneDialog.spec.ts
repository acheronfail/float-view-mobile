import { render } from 'svelte/server';
import { describe, expect, it, vi } from 'vitest';
import PruneDialog from './PruneDialog.svelte';

describe('PruneDialog', () => {
  it('explains preservation and offers a resolution control before preview', () => {
    const { body } = render(PruneDialog, {
      props: {
        open: true,
        scope: 'all',
        onpreview: vi.fn(),
        onprune: vi.fn(),
        oncancel: vi.fn(),
      },
    });

    expect(body).toContain('Prune all sessions?');
    expect(body).toContain('Target resolution');
    expect(body).toContain('minimum and maximum values');
    expect(body).toContain('ride-state transitions');
    expect(body).toContain('>Preview<');
  });
});
