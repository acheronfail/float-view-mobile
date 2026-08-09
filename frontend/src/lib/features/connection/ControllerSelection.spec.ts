import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { ActiveConnection } from '$lib/backend/contract';
import ControllerSelection from './ControllerSelection.svelte';

const connection: ActiveConnection = {
  deviceId: 'board',
  deviceKind: 'board',
  name: 'Board',
  displayName: 'Board',
  state: 'selecting_controller',
  trackingMode: 'saved',
  savedDeviceId: '0102030405060708090a0b0c',
  controllerConnection: null,
  controllerCandidates: [
    {
      canId: 42,
      uuid: '2a2a2a2a2a2a2a2a2a2a2a2a',
      hardwareName: 'THOR300',
      firmwareName: 'Refloat',
      vescVersion: '6.06',
      vescSupported: true,
    },
    {
      canId: 7,
      uuid: '070707070707070707070707',
      hardwareName: 'Old VESC',
      firmwareName: null,
      vescVersion: '5.03',
      vescSupported: false,
    },
  ],
  connectionIssue: null,
  bms: { status: 'discovering', uuid: null, connection: null, candidates: [], latestSnapshot: null },
  refloatControls: null,
  refloatConfiguration: null,
  controllerActions: { shutdownAvailable: false, pendingAction: null },
  firmware: null,
  latestSample: null,
  latestLocation: null,
  activeSession: null,
  currentLimits: null,
};

describe('controller selection prompt', () => {
  it('sorts candidates by CAN ID and leaves unsupported controllers visible but disabled', () => {
    const { body } = render(ControllerSelection, {
      props: { connection, ondisconnect: async () => undefined },
    });
    const text = body.replace(/\s+/g, ' ');

    expect(text).toContain('Choose a controller (2 found)');
    expect(text.indexOf('CAN 7')).toBeLessThan(text.indexOf('CAN 42'));
    expect(text).toContain('Old VESC · Unknown firmware · VESC 5.03');
    expect(text).toContain('THOR300 · Refloat · VESC 6.06');
    expect(body.match(/ disabled(?:>| )/g)).toHaveLength(1);
    expect(text).toContain('Unsupported');
    expect(text).toContain('Supported');
    expect(text).toContain('Disconnect');
    expect(body).toContain('fixed inset-0 z-40 h-screen bg-black/55');
    expect(body).toContain('absolute inset-x-0 bottom-0');
  });
});
