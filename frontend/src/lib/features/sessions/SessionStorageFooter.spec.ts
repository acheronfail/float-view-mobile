import { render } from 'svelte/server';
import { describe, expect, it, vi } from 'vitest';
import type { SavedDevice } from '$lib/backend/contract';
import SessionStorageFooter from './SessionStorageFooter.svelte';

const { state } = vi.hoisted(() => ({
  state: { snapshot: null as object | null, savedDevices: [] as SavedDevice[] },
}));
vi.mock('$lib/backend/state.svelte', () => ({ backendState: state }));
const board = (savedDeviceId: string, bytes: number | null) =>
  ({ savedDeviceId, summary: { totalStorageBytes: bytes } }) as SavedDevice;

describe('session storage footer', () => {
  it('shows loading before the first authoritative snapshot', () => {
    state.snapshot = null;
    expect(render(SessionStorageFooter).body).toContain('Loading session storage');
  });

  it('shows the entire catalog or selected board without depending on loaded pages', () => {
    state.snapshot = {};
    state.savedDevices = [board('one', 1024), board('two', 3072)];
    expect(render(SessionStorageFooter).body).toContain('Sessions on disk · 4.0 KB');
    const selected = render(SessionStorageFooter, { props: { savedDeviceId: 'two' } }).body;
    expect(selected).toContain('Sessions on disk · 3.0 KB');
    expect(selected).toContain('aria-label="Session storage"');
    expect(selected).toContain('border-t');
  });

  it('shows zero for an empty catalog and unavailable for a failed measurement', () => {
    state.snapshot = {};
    state.savedDevices = [];
    expect(render(SessionStorageFooter).body).toContain('Sessions on disk · 0 B');
    state.savedDevices = [board('one', null)];
    expect(render(SessionStorageFooter).body).toContain('Session storage unavailable');
  });
});
