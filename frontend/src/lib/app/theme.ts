import type { ThemePreference } from '$lib/backend/contract';

export type EffectiveTheme = 'light' | 'dark';

export function isThemePreference(value: unknown): value is ThemePreference {
	return value === 'system' || value === 'light' || value === 'dark';
}

export function resolveTheme(preference: ThemePreference, systemDark: boolean): EffectiveTheme {
	return preference === 'system' ? (systemDark ? 'dark' : 'light') : preference;
}

export interface ThemeEnvironment {
	systemDark(): boolean;
	apply(theme: EffectiveTheme): void;
	readHint(): unknown;
	writeHint(preference: ThemePreference): void;
	subscribe(listener: (systemDark: boolean) => void): () => void;
}

export class ThemeController {
	private preference: ThemePreference = 'system';
	private unsubscribe: (() => void) | null = null;

	constructor(private readonly environment: ThemeEnvironment) {}

	start() {
		const hint = this.environment.readHint();
		this.preference = isThemePreference(hint) ? hint : 'system';
		this.apply();
		this.unsubscribe = this.environment.subscribe(() => {
			if (this.preference === 'system') this.apply();
		});
		return () => {
			this.unsubscribe?.();
			this.unsubscribe = null;
		};
	}

	reconcile(preference: ThemePreference) {
		this.preference = preference;
		this.environment.writeHint(preference);
		this.apply();
	}

	private apply() {
		this.environment.apply(resolveTheme(this.preference, this.environment.systemDark()));
	}
}

export function browserThemeEnvironment(): ThemeEnvironment {
	const media = window.matchMedia('(prefers-color-scheme: dark)');
	return {
		systemDark: () => media.matches,
		apply: (theme) => {
			document.documentElement.dataset.theme = theme;
			document.documentElement.style.colorScheme = theme;
			document
				.querySelector<HTMLMetaElement>('meta[name="theme-color"]')
				?.setAttribute('content', theme === 'dark' ? '#020617' : '#f8fafc');
		},
		readHint: () => {
			try {
				return localStorage?.getItem('float-view-theme');
			} catch {
				return null;
			}
		},
		writeHint: (preference) => {
			try {
				localStorage?.setItem('float-view-theme', preference);
			} catch {
				// Theme persistence is optional.
			}
		},
		subscribe: (listener) => {
			const handler = (event: MediaQueryListEvent) => listener(event.matches);
			media.addEventListener('change', handler);
			return () => media.removeEventListener('change', handler);
		}
	};
}
