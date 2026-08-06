import { render } from 'svelte/server';
import { describe, expect, it, vi } from 'vitest';
import FlywheelModeOverlay from './FlywheelModeOverlay.svelte';

describe('flywheel mode overlay', () => {
  it('blocks the application with one flywheel-disable action and no dismissal control', () => {
    const { body } = render(FlywheelModeOverlay, { props: { ondisable: vi.fn() } });
    const text = body.replace(/\s+/g, ' ');

    expect(body).toContain('fixed inset-0 z-50');
    expect(body).toContain('role="alertdialog"');
    expect(body).toContain('aria-modal="true"');
    expect(text).toContain('Flywheel mode is active');
    expect(text).toContain('Disengage the board before disabling flywheel mode.');
    expect(text).toContain('>Disable</button>');
    expect(body.match(/<button/g)).toHaveLength(1);
    expect(text).not.toContain('Cancel');
  });

  it('disables its sole action while an authoritative request is unavailable', () => {
    const { body } = render(FlywheelModeOverlay, {
      props: { disabled: true, ondisable: vi.fn() },
    });

    expect(body).toMatch(/<button[^>]* disabled/);
  });
});
