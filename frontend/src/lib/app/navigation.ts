export const bottomTabs = [
  {
    label: 'connect',
    href: '#/',
    routeId: '/',
    emoji: '📶',
    title: 'Connect',
    subtitle: "Where's my board at?",
  },
  {
    label: 'sessions',
    href: '#/sessions',
    routeId: '/sessions',
    emoji: '📈',
    title: 'Sessions',
    subtitle: 'Where did I ride again?',
  },
  {
    label: 'boards',
    href: '#/boards',
    routeId: '/boards',
    emoji: '🛹',
    title: 'Boards',
    subtitle: 'Setup and manage your boards.',
  },
  {
    label: 'settings',
    href: '#/settings',
    routeId: '/settings',
    emoji: '🛠️',
    title: 'Settings',
    subtitle: 'Tinker and customize your experience.',
  },
] as const;

export type BottomTab = (typeof bottomTabs)[number];

export function bottomTabForRoute(routeId: string | null): BottomTab | undefined {
  return bottomTabs.find((tab) => tab.routeId === routeId);
}
