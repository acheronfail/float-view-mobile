import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { ActiveConnection } from '$lib/backend/contract';
import { emptyEstimate } from '../../../../backend-mock/battery';
import RefloatTunes from './RefloatTunes.svelte';

const connection: ActiveConnection = {
  deviceId: 'board',
  deviceKind: 'board',
  name: 'Board',
  displayName: 'Board',
  state: 'ready',
  trackingMode: 'guest',
  firmware: null,
  savedDeviceId: null,
  transportFirmware: null,
  controllerConnection: null,
  controllerCandidates: [],
  connectionIssue: null,
  bms: { status: 'discovering', connection: null, candidates: [], latestSnapshot: null },
  refloatControls: null,
  refloatConfiguration: {
    availability: 'ready',
    writable: true,
    packageVersion: '1.2.3',
    schemaFingerprint: 'a'.repeat(64),
    wireSignature: 123,
    currentLoaded: true,
    defaultLoaded: false,
    pendingOperation: null,
  },
  controllerActions: { shutdownAvailable: false, pendingAction: null },
  activeSession: null,
  latestSample: null,
  batteryEstimate: emptyEstimate(),
  liveWidgetSlots: ['empty', 'powerW', 'elapsedMs', 'odometerM', 'distanceM'],
  latestLocation: null,
  currentLimits: null,
};

describe('Refloat Tunes', () => {
  it('presents board presets without exposing configuration-editor drafts', () => {
    const body = render(RefloatTunes, { props: { connection } }).body;
    expect(body).toContain('Save tune');
    expect(body).not.toContain('View all saved tunes');
    expect(body).not.toContain('Show tunes for');
    expect(body).not.toContain('Load more tunes');
    expect(body).not.toContain('Tap a saved preset');
    expect(body).not.toContain('Only tunes matching');
    expect(body).not.toContain('Apply preset');
    expect(body).not.toContain('draft');
    expect(body).not.toContain('Update from');
  });
});
