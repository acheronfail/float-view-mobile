import { render } from 'svelte/server';
import { describe, expect, it, vi } from 'vitest';
import { backendState } from '$lib/backend/state.svelte';
import BottomTabBar from './BottomTabBar.svelte';
import { bottomTabForRoute, bottomTabs } from './navigation';

describe('bottom navigation', () => {
  it('maps the three exact labels to their top-level hash routes', () => {
    expect(bottomTabs.map(({ label, href, routeId }) => ({ label, href, routeId }))).toEqual([
      { label: 'connect', href: '#/', routeId: '/' },
      { label: 'sessions', href: '#/sessions', routeId: '/sessions' },
      { label: 'settings', href: '#/settings', routeId: '/settings' },
    ]);
  });

  it('selects only exact top-level routes', () => {
    expect(bottomTabForRoute('/')?.label).toBe('connect');
    expect(bottomTabForRoute('/sessions')?.label).toBe('sessions');
    expect(bottomTabForRoute('/boards')).toBeUndefined();
    expect(bottomTabForRoute('/settings')?.label).toBe('settings');
    expect(bottomTabForRoute('/live')).toBeUndefined();
    expect(bottomTabForRoute('/sessions/session-1')).toBeUndefined();
    expect(bottomTabForRoute('/boards/board-1')).toBeUndefined();
    expect(bottomTabForRoute('/unknown')).toBeUndefined();
    expect(bottomTabForRoute(null)).toBeUndefined();
  });

  it('renders icon-and-text links with the current tab selected', () => {
    const { body } = render(BottomTabBar, { props: { routeId: '/sessions' } });

    expect(body).toContain('data-app-navigation="bottom-tabs"');
    expect(body.match(/<svg/g)).toHaveLength(3);
    expect(body.match(/aria-current="page"/g)).toHaveLength(1);
    expect(body).toContain('href="#/sessions"');
    for (const { label } of bottomTabs) expect(body).toContain(`<span>${label}</span>`);
    expect(body).toContain('M12 3v18');
    expect(body).toContain('19.4 15');
    expect(body).not.toContain('>Devices<');
  });

  it.each(['/live', '/sessions/session-1', '/boards/board-1', '/unknown'])(
    'hides the tab bar on subordinate route %s',
    (routeId) => {
      const { body } = render(BottomTabBar, { props: { routeId } });
      expect(body).not.toContain('data-app-navigation="bottom-tabs"');
    }
  );

  it('leaves backend connection lifetime with the root shell across tab routes', () => {
    const connect = vi.spyOn(backendState, 'connect');
    const close = vi.spyOn(backendState, 'close');

    for (const { routeId } of bottomTabs) {
      render(BottomTabBar, { props: { routeId } });
    }

    expect(connect).not.toHaveBeenCalled();
    expect(close).not.toHaveBeenCalled();
    connect.mockRestore();
    close.mockRestore();
  });
});
