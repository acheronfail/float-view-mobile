import type { Snippet } from 'svelte';
import type { IconName } from '$lib/ui/icon';
import type { MenuItemAppearance } from '$lib/ui/MenuItem.svelte';

export type AppHeaderAction = {
  label: string;
  group?: string;
  groupDescription?: string;
  icon?: IconName;
  appearance?: MenuItemAppearance;
  disabled?: boolean;
  checked?: boolean | null;
  pending?: boolean;
  onactivate: (closeMenu: () => void) => void | Promise<void>;
};

export type AppHeaderPatch = {
  title?: string;
  subtitle?: string;
  subtitleSnippet?: Snippet;
  accessory?: Snippet;
  subtitleClass?: string;
  titleClass?: string;
  bottomBorder?: boolean;
  backHref?: string;
  backLabel?: string;
  menuLabel?: string;
  menuPresentation?: 'overlay';
  menuUnavailable?: boolean;
  menuWarning?: string;
  menu?: AppHeaderAction[];
};

export type AppHeaderConfig = AppHeaderPatch & {
  title: string;
};

export type AppHeaderContext = {
  set(owner: string, patch: AppHeaderPatch): void;
  clear(owner: string): void;
  openMenu(): void;
  focusMenu(): void;
};

export const APP_HEADER_CONTEXT = Symbol('app-header-context');
