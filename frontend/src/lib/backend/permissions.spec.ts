import { describe, it, expect } from 'vitest';
import { permissionInventory } from '../../../backend-mock/permissions';
import { validatePermissions } from './permissions';
import { MockBackend } from '../../../backend-mock/api-v1';

describe('permission inventories', () => {
  for (const platform of ['ios', 'macos', 'android29', 'android30', 'android31', 'android33', 'android36'] as const) {
    it(`decodes ${platform} without feature preferences or platform sniffing`, () => {
      const rows = permissionInventory(platform);
      expect(() => validatePermissions(rows)).not.toThrow();
      expect(rows.length).toBe(
        platform === 'android36'
          ? 5
          : ['android31', 'android33'].includes(platform)
            ? 4
            : platform.startsWith('android')
              ? 3
              : 2
      );
      expect(() => validatePermissions([...rows, rows[0]])).toThrow();
      for (const field of ['permission', 'status', 'action', 'guidance', 'accuracy', 'scope', 'scan']) {
        expect(() => validatePermissions([{ ...rows[0], [field]: 'unknown-value' }, ...rows.slice(1)])).toThrow();
      }
      expect(() => validatePermissions([{ ...rows[0], optionLabel: 'x'.repeat(121) }, ...rows.slice(1)])).toThrow();
    });
  }
});

it('mock recovery validates stale, inapplicable and pending actions without changing features', () => {
  const mock = new MockBackend();
  const state = () => mock.handle({ id: 1, method: 'state.get', params: {} });
  const initial = state();
  expect(
    mock.handle({
      id: 2,
      method: 'permissions.recover',
      params: { permission: 'location', action: 'requestPermission' },
    })
  ).toHaveProperty('result');
  expect(state()).toEqual(initial);
  expect(
    mock.handle({ id: 3, method: 'permissions.recover', params: { permission: 'location', action: 'openSettings' } })
  ).toHaveProperty('error.code', 'invalid_state');
  expect(
    mock.handle({
      id: 4,
      method: 'permissions.recover',
      params: { permission: 'backgroundLocation', action: 'requestPermission' },
    })
  ).toHaveProperty('error.code', 'invalid_state');
  expect(
    mock.handle({ id: 5, method: 'permissions.recover', params: { permission: 'location', action: 'navigate' } })
  ).toHaveProperty('error.code', 'invalid_request');
  mock.simulatePermissions(
    permissionInventory('android31').map((row) => ({ ...row, pending: row.permission === 'location' }))
  );
  expect(
    mock.handle({
      id: 6,
      method: 'permissions.recover',
      params: { permission: 'location', action: 'requestPermission' },
    })
  ).toHaveProperty('error.code', 'conflict');
});

it('recovers notifications and promotion only through advertised actions', () => {
  const mock = new MockBackend();
  const rows = permissionInventory('android36');
  mock.simulatePermissions(rows);
  expect(
    mock.handle({
      id: 1,
      method: 'permissions.recover',
      params: { permission: 'notifications', action: 'requestPermission' },
    })
  ).toHaveProperty('result');
  expect(
    mock.handle({
      id: 2,
      method: 'permissions.recover',
      params: { permission: 'liveUpdates', action: 'requestPermission' },
    })
  ).toHaveProperty('error.code', 'invalid_state');
  mock.simulatePermissions(
    rows.map((row) =>
      row.permission === 'notifications'
        ? { ...row, status: 'granted', action: 'openSettings', guidance: 'notificationSettings' }
        : row.permission === 'liveUpdates'
          ? { ...row, action: 'openSettings', guidance: 'liveUpdateSettings' }
          : row
    )
  );
  expect(
    mock.handle({ id: 3, method: 'permissions.recover', params: { permission: 'liveUpdates', action: 'openSettings' } })
  ).toHaveProperty('result');
});
