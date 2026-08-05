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
    subtitle: 'Where did I ride again?',
  },
  {
    label: 'settings',
    href: '#/settings',
    routeId: '/settings',
    icon: 'settings',
    title: 'Settings',
    subtitle: 'Tinker and customize your experience.',
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

export function bottomTabForRoute(routeId: string | null): BottomTab | undefined {
  return bottomTabs.find((tab) => tab.routeId === routeId);
}
