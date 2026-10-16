import type { AccentPreference, ThemePreference } from '$lib/backend/contract';

export const accentPresets: { value: AccentPreference; label: string }[] = [
  { value: 'purple', label: 'Purple' },
  { value: 'blue', label: 'Blue' },
  { value: 'teal', label: 'Teal' },
  { value: 'amber', label: 'Amber' },
  { value: 'rose', label: 'Rose' },
];

export function isAccentPreference(value: unknown): value is AccentPreference {
  return accentPresets.some((preset) => preset.value === value);
}

export type EffectiveTheme = 'light' | 'dark';

export function isThemePreference(value: unknown): value is ThemePreference {
  return value === 'system' || value === 'light' || value === 'dark';
}

export function resolveTheme(preference: ThemePreference, systemDark: boolean): EffectiveTheme {
  return preference === 'system' ? (systemDark ? 'dark' : 'light') : preference;
}

export interface ThemeEnvironment {
  systemDark(): boolean;
  apply(theme: EffectiveTheme, accent: AccentPreference): void;
  readHint(): unknown;
  readAccentHint(): unknown;
  writeHint(preference: ThemePreference, accent: AccentPreference): void;
  subscribe(listener: (systemDark: boolean) => void): () => void;
}

export class ThemeController {
  private preference: ThemePreference = 'system';
  private accent: AccentPreference = 'purple';
  private systemDark = false;
  private unsubscribe: (() => void) | null = null;

  constructor(private readonly environment: ThemeEnvironment) {}

  start() {
    const hint = this.environment.readHint();
    this.preference = isThemePreference(hint) ? hint : 'system';
    const accentHint = this.environment.readAccentHint();
    this.accent = isAccentPreference(accentHint) ? accentHint : 'purple';
    this.systemDark = this.environment.systemDark();
    this.apply();
    this.unsubscribe = this.environment.subscribe((systemDark) => {
      this.systemDark = systemDark;
      if (this.preference === 'system') this.apply();
    });
    return () => {
      this.unsubscribe?.();
      this.unsubscribe = null;
    };
  }

  reconcile(preference: ThemePreference, accent: AccentPreference = 'purple') {
    this.preference = preference;
    this.accent = accent;
    this.environment.writeHint(preference, accent);
    this.apply();
  }

  private apply() {
    this.environment.apply(resolveTheme(this.preference, this.systemDark), this.accent);
  }
}

export function browserThemeEnvironment(): ThemeEnvironment {
  const media = window.matchMedia('(prefers-color-scheme: dark)');
  return {
    systemDark: () => media.matches,
    apply: (theme, accent) => {
      document.documentElement.dataset.accent = accent;
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
    readAccentHint: () => {
      try {
        return localStorage?.getItem('float-view-accent');
      } catch {
        return null;
      }
    },
    writeHint: (preference, accent) => {
      try {
        localStorage?.setItem('float-view-theme', preference);
        localStorage?.setItem('float-view-accent', accent);
      } catch {
        // Theme persistence is optional.
      }
    },
    subscribe: (listener) => {
      const mediaHandler = (event: MediaQueryListEvent) => listener(event.matches);
      const reconcile = () => listener(media.matches);
      const visibilityHandler = () => {
        if (document.visibilityState === 'visible') reconcile();
      };

      media.addEventListener('change', mediaHandler);
      window.addEventListener('focus', reconcile);
      window.addEventListener('pageshow', reconcile);
      document.addEventListener('visibilitychange', visibilityHandler);
      return () => {
        media.removeEventListener('change', mediaHandler);
        window.removeEventListener('focus', reconcile);
        window.removeEventListener('pageshow', reconcile);
        document.removeEventListener('visibilitychange', visibilityHandler);
      };
    },
  };
}
