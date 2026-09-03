import { render } from 'svelte/server';
import { describe, expect, it, vi } from 'vitest';
import InformationDialog from './InformationDialog.svelte';
import { restoreDialogFocus, wrappedDialogFocus } from './dialog';

describe('InformationDialog', () => {
  it('renders non-urgent modal help with one explicit action and a dismissing backdrop', () => {
    const { body } = render(InformationDialog, {
      props: {
        open: true,
        titleId: 'field-help-title',
        title: 'Tiltback Constant',
        description: 'Controls constant tiltback.',
        onclose: vi.fn(),
      },
    });

    expect(body).toContain('role="dialog"');
    expect(body).toContain('aria-modal="true"');
    expect(body).toContain('aria-labelledby="field-help-title"');
    expect(body).toContain('aria-describedby="field-help-title-description"');
    expect(body).toContain('aria-label="Close help"');
    expect(body).toContain('Tiltback Constant');
    expect(body).toContain('Controls constant tiltback.');
    expect(body).toContain('>Done</button>');
    expect(body.match(/<button/g)).toHaveLength(2);
    expect(body).not.toContain('role="alertdialog"');
  });

  it('wraps focus within the dialog in either direction', () => {
    const first = { name: 'first' };
    const middle = { name: 'middle' };
    const last = { name: 'last' };
    const controls = [first, middle, last];

    expect(wrappedDialogFocus(first, controls, true)).toBe(last);
    expect(wrappedDialogFocus(last, controls, false)).toBe(first);
    expect(wrappedDialogFocus(middle, controls, false)).toBeNull();
    expect(wrappedDialogFocus(first, [], true)).toBeNull();
  });

  it('restores focus to the provided trigger', () => {
    const focus = vi.fn();
    restoreDialogFocus({ focus });
    restoreDialogFocus(undefined);
    expect(focus).toHaveBeenCalledOnce();
  });
});
