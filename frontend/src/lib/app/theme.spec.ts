import type { AccentPreference } from '$lib/backend/contract';
import { describe, expect, it, vi } from 'vitest';
import {
  ThemeController,
  isAccentPreference,
  isThemePreference,
  resolveTheme,
  type EffectiveTheme,
  type ThemeEnvironment,
} from './theme';

function environment(hint: unknown, systemDark = false, accentHint: unknown = null) {
  let dark = systemDark;
  let listener: ((value: boolean) => void) | undefined;
  const applied: EffectiveTheme[] = [];
  const accents: AccentPreference[] = [];
  const written: string[] = [];
  const value: ThemeEnvironment = {
    systemDark: () => dark,
    apply: (theme, accent) => {
      applied.push(theme);
      accents.push(accent);
    },
    readHint: () => hint,
    readAccentHint: () => accentHint,
    writeHint: (preference) => written.push(preference),
    subscribe: (next) => {
      listener = next;
      return vi.fn();
    },
  };
  return {
    value,
    applied,
    accents,
    written,
    changeSystem(value: boolean) {
      dark = value;
      listener?.(value);
    },
  };
}

describe('theme resolution', () => {
  it('accepts only the complete preference set', () => {
    expect(['system', 'light', 'dark'].every(isThemePreference)).toBe(true);
    expect(isThemePreference('twilight')).toBe(false);
  });

  it('resolves explicit and system preferences', () => {
    expect(resolveTheme('system', false)).toBe('light');
    expect(resolveTheme('system', true)).toBe('dark');
    expect(resolveTheme('light', true)).toBe('light');
    expect(resolveTheme('dark', false)).toBe('dark');
  });

  it('reconciles a hint to authoritative state and follows system only when selected', () => {
    const test = environment('dark');
    const controller = new ThemeController(test.value);
    controller.start();
    expect(test.applied).toEqual(['dark']);
    controller.reconcile('system');
    expect(test.written).toEqual(['system']);
    expect(test.applied.at(-1)).toBe('light');
    test.changeSystem(true);
    expect(test.applied.at(-1)).toBe('dark');
    controller.reconcile('light');
    test.changeSystem(false);
    expect(test.applied.at(-1)).toBe('light');
  });

  it('remembers system changes received while an explicit theme is selected', () => {
    const test = environment('light');
    const controller = new ThemeController(test.value);
    controller.start();
    test.changeSystem(true);
    expect(test.applied).toEqual(['light']);
    controller.reconcile('system');
    expect(test.applied.at(-1)).toBe('dark');
  });
});

describe('accent preferences', () => {
  it('accepts only the five presets', () => {
    expect(['purple', 'blue', 'teal', 'amber', 'rose'].every(isAccentPreference)).toBe(true);
    for (const value of [null, 1, 'orange', '']) expect(isAccentPreference(value)).toBe(false);
  });

  it('reconciles the startup hint and preserves the accent on system theme changes', () => {
    const test = environment('system', false, 'rose');
    const controller = new ThemeController(test.value);
    controller.start();
    expect(test.accents).toEqual(['rose']);
    controller.reconcile('system', 'teal');
    test.changeSystem(true);
    expect(test.accents).toEqual(['rose', 'teal', 'teal']);
    expect(test.applied.at(-1)).toBe('dark');
  });

  it.each([null, 'orange'])('uses teal for missing or invalid startup hint %s', (hint) => {
    const test = environment('dark', false, hint);
    const controller = new ThemeController(test.value);
    controller.start();
    controller.reconcile('system');
    expect(test.accents).toEqual(['teal', 'teal']);
  });

  it('preserves a saved purple choice after the default changes', () => {
    const test = environment('system', false, 'purple');
    const controller = new ThemeController(test.value);
    controller.start();
    controller.reconcile('system', 'purple');
    expect(test.accents).toEqual(['purple', 'purple']);
  });
});
