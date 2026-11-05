import { render } from 'svelte/server';
import { describe, expect, it, vi } from 'vitest';
import type { SavedDevice, UnitPreference } from '$lib/backend/contract';
import SessionTotals from './SessionTotals.svelte';

const { state } = vi.hoisted(() => ({
  state: {
    snapshot: {} as object | null,
    savedDevices: [] as SavedDevice[],
    settings: { unitPreference: 'metric' as UnitPreference },
  },
}));
vi.mock('$lib/backend/state.svelte', () => ({ backendState: state }));
const board = (sessionCount: number, totalRidingElapsedMs: number, totalDistanceM: number | null) =>
  ({ summary: { sessionCount, totalRidingElapsedMs, totalDistanceM } }) as SavedDevice;

describe('session totals', () => {
  it('aggregates the entire catalog and converts distance to the selected units', () => {
    state.snapshot = {};
    state.savedDevices = [board(4, 3_600_000, 1609.344), board(2, 1_800_000, 1609.344), board(1, 0, null)];
    state.settings.unitPreference = 'metric';
    const metric = render(SessionTotals).body;
    expect(metric).toContain('3.22 km');
    expect(metric).toContain('1.5 h');
    expect(metric).toMatch(/>\s*7\s*</);
    state.settings.unitPreference = 'imperial';
    expect(render(SessionTotals).body).toContain('2.00 mi');
  });

  it('distinguishes empty history, unavailable distance and loading', () => {
    state.snapshot = {};
    state.settings.unitPreference = 'metric';
    state.savedDevices = [];
    expect(render(SessionTotals).body).toContain('0.00 km');
    state.savedDevices = [board(1, 0, null)];
    expect(render(SessionTotals).body).not.toContain('0.00 km');
    expect(render(SessionTotals).body).toContain('—');
    state.snapshot = null;
    expect(render(SessionTotals).body).not.toContain('0.0 h');
  });
});
