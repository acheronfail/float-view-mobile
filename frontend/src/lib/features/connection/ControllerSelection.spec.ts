import { emptyEstimate } from '../../../../backend-mock/battery';
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
  savedDeviceId: 'saved-board',
  transportFirmware: null,
  controllerConnection: null,
  controllerCandidates: [
    {
      canId: 42,
      hardwareName: 'THOR300',
      firmwareName: 'Refloat',
      vescVersion: '6.06',
      vescSupported: true,
    },
    {
      canId: 7,
      hardwareName: 'Old VESC',
      firmwareName: null,
      vescVersion: '5.03',
      vescSupported: false,
    },
  ],
  connectionIssue: null,
  connectionWarning: null,
  bms: { status: 'discovering', connection: null, candidates: [], latestSnapshot: null },
  refloatControls: null,
  refloatConfiguration: null,
  controllerActions: { shutdownAvailable: false, pendingAction: null },
  firmware: null,
  latestSample: null,
  batteryEstimate: emptyEstimate(),
  liveWidgetSlots: ['empty', 'powerW', 'elapsedMs', 'odometerM', 'distanceM'],
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

    expect(text).toContain('Choose controller');
    expect(text.indexOf('CAN 7')).toBeLessThan(text.indexOf('CAN 42'));
    expect(text).toContain('Old VESC · VESC 5.03');
    expect(text).toContain('THOR300 · Refloat · VESC 6.06');
    expect(body.match(/ disabled(?:>| )/g)).toHaveLength(1);
    expect(text).toContain('VESC not supported');
    expect(text).toContain('Connect');
    expect(text).toContain('2 CAN addresses responded');
    expect(text).toContain('check the controller’s CAN ID in VESC Tool');
    expect(text).not.toContain('Refloat compatibility is checked after');
    expect(text).not.toContain('Matching names and versions');
    expect(text).not.toContain('Unknown firmware');
    expect(text).toContain('Disconnect');
    expect(body).toContain('fixed inset-0 z-40 h-dvh w-screen bg-canvas');
  });
});
