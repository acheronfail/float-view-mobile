export const bottomTabs = [
  { label: 'connect', href: '#/', routeId: '/', icon: 'connect' },
  { label: 'sessions', href: '#/sessions', routeId: '/sessions', icon: 'sessions' },
  { label: 'boards', href: '#/boards', routeId: '/boards', icon: 'boards' },
  { label: 'settings', href: '#/settings', routeId: '/settings', icon: 'settings' },
] as const;

export type BottomTab = (typeof bottomTabs)[number];

export function bottomTabForRoute(routeId: string | null): BottomTab | undefined {
  return bottomTabs.find((tab) => tab.routeId === routeId);
}
