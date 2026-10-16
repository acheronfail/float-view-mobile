import type { ActiveConnection } from '$lib/backend/contract';

export type ConnectionRoute = '/live' | '/bms';
export const connectionReturnHref = '#/sessions';

export function connectionRoute(connection: ActiveConnection | null | undefined): ConnectionRoute | null {
  if (connection?.deviceKind === 'bms') return '/bms';
  if (connection?.deviceKind === 'board') return '/live';
  return null;
}

export function connectionHref(connection: ActiveConnection | null | undefined): '#/live' | '#/bms' | null {
  const route = connectionRoute(connection);
  return route ? `#${route}` : null;
}

export function connectionRedirect(
  routeId: string | null,
  connection: ActiveConnection | null | undefined,
  backendConnected: boolean
): '#/' | '#/live' | '#/bms' | null {
  if (!backendConnected) return null;
  if (!connection) return routeId === '/live' || routeId === '/bms' ? '#/' : null;
  const target = connectionHref(connection);
  if (!target) return routeId === '/' ? '#/live' : null;
  if (routeId === '/') return target;
  if (routeId === '/live' && target === '#/bms') return target;
  if (routeId === '/bms' && target === '#/live') return target;
  return null;
}
