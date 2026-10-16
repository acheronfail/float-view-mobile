import type { IconName } from '$lib/ui/icon';

export const bottomTabs = [
  {
    label: 'connect',
    href: '#/',
    routeId: '/',
    icon: 'bluetooth',
    title: 'Connect',
    subtitle: "Where's my board at?",
  },
  {
    label: 'sessions',
    href: '#/sessions',
    routeId: '/sessions',
    icon: 'route',
    title: 'Sessions',
    subtitle: 'Loading rides…',
  },
  {
    label: 'settings',
    href: '#/settings',
    routeId: '/settings',
    icon: 'settings',
    title: 'Settings',
    subtitle: 'Loading ride totals…',
  },
] as const satisfies readonly {
  label: string;
  href: string;
  routeId: string;
  icon: IconName;
  title: string;
  subtitle: string;
}[];

export type BottomTab = (typeof bottomTabs)[number];

/** SvelteKit's hash router keeps route queries inside the fragment. */
export function routeSearchParams(url: URL): URLSearchParams {
  return new URL(url.hash.slice(1) || '/', url).searchParams;
}

export function bottomTabForRoute(routeId: string | null): BottomTab | undefined {
  return bottomTabs.find((tab) => tab.routeId === routeId);
}
