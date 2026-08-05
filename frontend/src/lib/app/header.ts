import type { Snippet } from 'svelte';
import type { MenuItemAppearance } from '$lib/ui/MenuItem.svelte';

export type AppHeaderAction = {
  label: string;
  appearance?: MenuItemAppearance;
  disabled?: boolean;
  onactivate: (closeMenu: () => void) => void | Promise<void>;
};

export type AppHeaderPatch = {
  title?: string;
  subtitle?: string;
  subtitleSnippet?: Snippet;
  subtitleClass?: string;
  titleClass?: string;
  bottomBorder?: boolean;
  backHref?: string;
  backLabel?: string;
  menuLabel?: string;
  menu?: AppHeaderAction[];
};

export type AppHeaderConfig = AppHeaderPatch & {
  title: string;
};

export type AppHeaderContext = {
  set(owner: string, patch: AppHeaderPatch): void;
  clear(owner: string): void;
  focusMenu(): void;
};

export const APP_HEADER_CONTEXT = Symbol('app-header-context');
