import { lightingBox } from './lighting';
import { emptyEstimate } from '../../../../backend-mock/battery';
import { describe, expect, it } from 'vitest';
import type { ActiveConnection } from '$lib/backend/contract';
import {
  controllerShutdownConfirmation,
  controllerShutdownDisabled,
  controllerRouteDescription,
  flywheelDisableUnavailable,
  flywheelOverlayVisible,
  firmwareAccessDescription,
  readingsAreDimmed,
  refloatControlConfirmation,
  refloatMenuActions,
  shouldConfirmDisconnect,
  sortedControllerCandidates,
} from './live';

const refloatControls = {
  status: 'ready' as const,
  writable: true,
  packageEnabled: true,
  flywheelEnabled: false,
  lightingAvailable: true,
  ledsEnabled: true,
  headlightsEnabled: false,
  fatalErrorClearAvailable: true,
  fatalErrorLatched: true,
  fatalErrorActive: false,
  fatalErrorClearable: true,
  pendingAction: null,
};

const connection = (state: ActiveConnection['state']): ActiveConnection => ({
  deviceId: 'board',
  deviceKind: 'board',
  name: 'Board',
  displayName: 'Board',
  state,
  trackingMode: 'guest',
  firmware: null,
  savedDeviceId: null,
  controllerConnection: null,
  controllerCandidates: [],
  connectionIssue: null,
  bms: { status: 'discovering', connection: null, candidates: [], latestSnapshot: null },
  refloatControls: null,
  refloatConfiguration: null,
  controllerActions: { shutdownAvailable: false, pendingAction: null },
  activeSession: null,
  latestSample: null,
  batteryEstimate: emptyEstimate(),
  latestLocation: null,
  currentLimits: null,
});

describe('live board behavior', () => {
  it('dims retained readings for every state except ready', () => {
    expect(readingsAreDimmed(connection('ready'))).toBe(false);
    expect(readingsAreDimmed(connection('reconnecting'))).toBe(true);
    expect(readingsAreDimmed(connection('disconnecting'))).toBe(true);
  });

  it('skips confirmation only for an exactly idle latest sample', () => {
    expect(shouldConfirmDisconnect('idle')).toBe(false);
    expect(shouldConfirmDisconnect('riding')).toBe(true);
    expect(shouldConfirmDisconnect('unknown')).toBe(true);
    expect(shouldConfirmDisconnect(undefined)).toBe(true);
  });

  it('describes direct and CAN routes without exposing private identity', () => {
    expect(controllerRouteDescription({ kind: 'direct' })).toBe('Direct Bluetooth');
    expect(controllerRouteDescription({ kind: 'can', canId: 42 })).toBe('CAN 42');
    expect(controllerRouteDescription(null)).toBe('Detecting…');
  });

  it('labels audited test firmware as read-only', () => {
    expect(
      firmwareAccessDescription({
        vescVersion: '7.00',
        packageName: 'refloat',
        packageVersion: '1.2.3',
        hardwareName: 'THOR300',
        accessMode: 'readOnly',
        supported: true,
      })
    ).toBe('Read-only (test firmware)');
  });

  it('sorts supported and unsupported candidates by CAN ID without hiding either', () => {
    const candidates = sortedControllerCandidates([
      {
        canId: 42,
        hardwareName: 'THOR300',
        firmwareName: 'Refloat',
        vescVersion: '6.06',
        vescSupported: true,
      },
      {
        canId: 7,
        hardwareName: 'Older controller',
        firmwareName: null,
        vescVersion: '5.03',
        vescSupported: false,
      },
    ]);
    expect(candidates.map(({ canId, vescSupported }) => [canId, vescSupported])).toEqual([
      [7, false],
      [42, true],
    ]);
  });

  it('derives ready, riding, read-only, reconnecting, lighting, fatal, and pending menu gates', () => {
    const ready = connection('ready');
    ready.refloatControls = { ...refloatControls };
    ready.latestSample = { rideState: 'idle', packageState: 'ready' } as ActiveConnection['latestSample'];
    expect(refloatMenuActions(ready).map((item) => [item.label, item.disabled])).toEqual([
      ['Disable Refloat…', false],
      ['Enable flywheel…', false],
      ['Clear fatal error', false],
    ]);

    ready.latestSample = { rideState: 'riding', packageState: 'running' } as ActiveConnection['latestSample'];
    const riding = refloatMenuActions(ready);
    expect(lightingBox(ready, 'leds').disabled).toBe(false);
    expect(riding.find((item) => item.label === 'Disable Refloat…')?.disabled).toBe(true);
    expect(riding.find((item) => item.label === 'Clear fatal error')?.disabled).toBe(true);

    ready.refloatControls = { ...refloatControls, writable: false };
    expect(lightingBox(ready, 'leds').disabled).toBe(true);
    expect(refloatMenuActions(ready).every((item) => item.disabled)).toBe(true);
    ready.refloatControls = { ...refloatControls };
    ready.state = 'reconnecting';
    expect(lightingBox(ready, 'headlights').disabled).toBe(true);
    expect(refloatMenuActions(ready).every((item) => item.disabled)).toBe(true);

    ready.state = 'ready';
    ready.latestSample = { rideState: 'idle', packageState: 'ready' } as ActiveConnection['latestSample'];
    ready.refloatControls = {
      ...refloatControls,
      lightingAvailable: false,
      fatalErrorActive: true,
      fatalErrorClearable: false,
      pendingAction: 'package',
    };
    expect(lightingBox(ready, 'leds')).toMatchObject({ checked: null, disabled: true, status: 'Unavailable' });
    const unavailable = refloatMenuActions(ready);
    expect(unavailable.some((item) => item.label.startsWith('LEDs'))).toBe(false);
    expect(unavailable.find((item) => item.kind === 'clearFatalError')?.disabled).toBe(true);
    expect(unavailable.every((item) => item.disabled)).toBe(true);
  });

  it('keeps confirmed lighting state and stable labels during pending and unknown states', () => {
    const ready = connection('ready');
    ready.latestSample = { rideState: 'idle', packageState: 'ready' } as ActiveConnection['latestSample'];
    ready.refloatControls = { ...refloatControls, pendingAction: 'leds' };
    expect(lightingBox(ready, 'leds')).toMatchObject({
      checked: true,
      pending: true,
      disabled: true,
    });
    ready.refloatControls = { ...refloatControls, ledsEnabled: null, headlightsEnabled: false };
    expect(lightingBox(ready, 'leds')).toMatchObject({
      checked: null,
      disabled: true,
    });
    expect(lightingBox(ready, 'headlights')).toMatchObject({
      checked: false,
      disabled: false,
    });
    ready.refloatControls.packageEnabled = false;
    expect(refloatMenuActions(ready)[0]).toMatchObject({ label: 'Enable Refloat', enabled: true });
  });

  it('moves flywheel disable out of the menu while flywheel mode is active', () => {
    const ready = connection('ready');
    ready.refloatControls = { ...refloatControls, flywheelEnabled: true };
    ready.latestSample = {
      rideState: 'idle',
      packageState: 'ready',
      packageMode: 'flywheel',
    } as ActiveConnection['latestSample'];
    expect(lightingBox(ready, 'leds').disabled).toBe(true);
    expect(refloatMenuActions(ready)).toEqual([]);
    expect(flywheelOverlayVisible(ready)).toBe(true);
    expect(flywheelDisableUnavailable(ready)).toBe(false);

    ready.latestSample = { ...ready.latestSample, rideState: 'riding' } as ActiveConnection['latestSample'];
    expect(flywheelDisableUnavailable(ready)).toBe(false);

    ready.refloatControls = { ...ready.refloatControls, pendingAction: 'flywheel' };
    expect(flywheelOverlayVisible(ready)).toBe(true);
    expect(flywheelDisableUnavailable(ready)).toBe(true);

    ready.refloatControls = { ...ready.refloatControls, flywheelEnabled: false, pendingAction: null };
    expect(flywheelOverlayVisible(ready)).toBe(false);
  });

  it('requires contextual confirmation only for package disable and flywheel enable', () => {
    expect(refloatControlConfirmation('package', false)).toMatchObject({
      confirmLabel: 'Disable package',
      description: expect.stringContaining('Balancing will be disabled'),
    });
    expect(refloatControlConfirmation('flywheel', true)).toMatchObject({
      confirmLabel: 'Enable flywheel',
      description: expect.stringContaining('nose-up'),
    });
    expect(refloatControlConfirmation('package', true)).toBeNull();
    expect(refloatControlConfirmation('flywheel', false)).toBeNull();
    expect(refloatControlConfirmation('leds', false)).toBeNull();
  });

  it('gates shutdown from authoritative controller action state and warns about no-op controllers', () => {
    const ready = connection('ready');
    expect(controllerShutdownDisabled(ready)).toBe(true);

    ready.controllerActions = { shutdownAvailable: true, pendingAction: null };
    expect(controllerShutdownDisabled(ready)).toBe(false);

    ready.controllerActions = { shutdownAvailable: false, pendingAction: 'shutdown' };
    expect(controllerShutdownDisabled(ready)).toBe(true);
    expect(controllerShutdownConfirmation).toEqual({
      title: 'Shut down controller?',
      description: expect.stringContaining('Not all controllers support software shutdown'),
      confirmLabel: 'Shut down',
    });
  });
});
