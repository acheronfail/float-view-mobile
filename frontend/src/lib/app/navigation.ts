export const bottomTabs = [
  {
    label: 'connect',
    href: '#/',
    routeId: '/',
    icon: 'connect',
    title: 'Connect',
    subtitle: "Where's my board at?",
  },
  {
    label: 'sessions',
    href: '#/sessions',
    routeId: '/sessions',
    icon: 'sessions',
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
] as const;

export type BottomTab = (typeof bottomTabs)[number];

export function bottomTabForRoute(routeId: string | null): BottomTab | undefined {
  return bottomTabs.find((tab) => tab.routeId === routeId);
}
