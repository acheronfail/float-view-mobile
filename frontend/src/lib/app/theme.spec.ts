import { describe, expect, it, vi } from 'vitest';
import {
	ThemeController,
	isThemePreference,
	resolveTheme,
	type EffectiveTheme,
	type ThemeEnvironment
} from './theme';

function environment(hint: unknown, systemDark = false) {
	let dark = systemDark;
	let listener: ((value: boolean) => void) | undefined;
	const applied: EffectiveTheme[] = [];
	const written: string[] = [];
	const value: ThemeEnvironment = {
		systemDark: () => dark,
		apply: (theme) => applied.push(theme),
		readHint: () => hint,
		writeHint: (preference) => written.push(preference),
		subscribe: (next) => {
			listener = next;
			return vi.fn();
		}
	};
	return {
		value,
		applied,
		written,
		changeSystem(value: boolean) {
			dark = value;
			listener?.(value);
		}
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
});
