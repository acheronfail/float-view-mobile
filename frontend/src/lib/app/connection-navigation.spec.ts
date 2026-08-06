import { describe, expect, it } from 'vitest';
import type { ActiveConnection } from '$lib/backend/contract';
import { connectionHref, connectionRedirect, connectionRoute } from './connection-navigation';

const connection = (deviceKind: ActiveConnection['deviceKind']) => ({ deviceKind }) as ActiveConnection;

describe('connection navigation', () => {
  it('routes only authoritative classified connection kinds', () => {
    expect(connectionRoute(connection('board'))).toBe('/live');
    expect(connectionHref(connection('board'))).toBe('#/live');
    expect(connectionRoute(connection('bms'))).toBe('/bms');
    expect(connectionHref(connection('bms'))).toBe('#/bms');
    expect(connectionRoute(connection(null))).toBeNull();
    expect(connectionRoute(null)).toBeNull();
  });

  it('restores and guards connection routes from complete state', () => {
    expect(connectionRedirect('/', connection('bms'), true)).toBe('#/bms');
    expect(connectionRedirect('/', connection('board'), true)).toBe('#/live');
    expect(connectionRedirect('/', connection(null), true)).toBe('#/live');
    expect(connectionRedirect('/live', connection(null), true)).toBeNull();
    expect(connectionRedirect('/bms', connection(null), true)).toBeNull();
    expect(connectionRedirect('/live', connection('bms'), true)).toBe('#/bms');
    expect(connectionRedirect('/bms', connection('board'), true)).toBe('#/live');
    expect(connectionRedirect('/bms', null, true)).toBe('#/');
    expect(connectionRedirect('/live', null, true)).toBe('#/');
    expect(connectionRedirect('/bms', connection('bms'), true)).toBeNull();
    expect(connectionRedirect('/', connection('bms'), false)).toBeNull();
  });
});
