import type {
  ActiveConnection,
  ControllerCandidate,
  ControllerConnection,
  FirmwareInfo,
  RideState,
  RefloatControl,
} from '$lib/backend/contract';

export type RefloatMenuAction =
  | { kind: 'control'; control: RefloatControl; enabled: boolean; label: string; disabled: boolean }
  | { kind: 'clearFatalError'; label: string; disabled: boolean };

export interface RefloatControlConfirmation {
  control: RefloatControl;
  enabled: boolean;
  title: string;
  description: string;
  confirmLabel: string;
}

export const controllerShutdownConfirmation = {
  title: 'Shut down controller?',
  description:
    'Not all controllers support software shutdown. If this controller does not support it, nothing will happen. Make sure the board is idle before continuing.',
  confirmLabel: 'Shut down',
} as const;

export function controllerShutdownDisabled(connection: ActiveConnection): boolean {
  return !connection.controllerActions.shutdownAvailable || connection.controllerActions.pendingAction !== null;
}

export function flywheelOverlayVisible(connection: ActiveConnection | null | undefined): boolean {
  return connection?.refloatControls?.flywheelEnabled === true;
}

export function flywheelDisableUnavailable(connection: ActiveConnection | null | undefined): boolean {
  const controls = connection?.refloatControls;
  return (
    connection?.state !== 'ready' ||
    controls?.status !== 'ready' ||
    !controls.writable ||
    controls.pendingAction !== null
  );
}

export function refloatControlConfirmation(
  control: RefloatControl,
  enabled: boolean
): RefloatControlConfirmation | null {
  if (control === 'package' && !enabled) {
    return {
      control,
      enabled,
      title: 'Disable Refloat package?',
      description: 'Balancing will be disabled. Keep the board disengaged and supported before continuing.',
      confirmLabel: 'Disable package',
    };
  }
  if (control === 'flywheel' && enabled) {
    return {
      control,
      enabled,
      title: 'Enable flywheel?',
      description: 'Keep the board disengaged, nose-up, and balanced before enabling flywheel mode.',
      confirmLabel: 'Enable flywheel',
    };
  }
  return null;
}

export function refloatMenuActions(connection: ActiveConnection): RefloatMenuAction[] {
  const controls = connection.refloatControls;
  if (!controls || controls.status === 'unavailable') return [];
  const actionable =
    connection.state === 'ready' && controls.status === 'ready' && controls.writable && controls.pendingAction === null;
  const idle = connection.latestSample?.rideState === 'idle';
  const flywheel = controls.flywheelEnabled === true;
  const stateLabel = (label: string, value: boolean | null) =>
    `${label}: ${value === null ? 'Unknown' : value ? 'On' : 'Off'}`;

  if (flywheel) return [];

  const actions: RefloatMenuAction[] = [
    {
      kind: 'control',
      control: 'package',
      enabled: !(controls.packageEnabled ?? true),
      label: stateLabel('Refloat package', controls.packageEnabled),
      disabled: !actionable || !idle || controls.packageEnabled === null,
    },
    {
      kind: 'control',
      control: 'flywheel',
      enabled: !(controls.flywheelEnabled ?? false),
      label: stateLabel('Flywheel', controls.flywheelEnabled),
      disabled:
        !actionable ||
        !idle ||
        controls.flywheelEnabled === null ||
        (controls.flywheelEnabled === false && connection.latestSample?.packageState !== 'ready'),
    },
  ];
  if (controls.lightingAvailable) {
    actions.push(
      {
        kind: 'control',
        control: 'leds',
        enabled: !(controls.ledsEnabled ?? true),
        label: stateLabel('LEDs', controls.ledsEnabled),
        disabled: !actionable || controls.ledsEnabled === null,
      },
      {
        kind: 'control',
        control: 'headlights',
        enabled: !(controls.headlightsEnabled ?? true),
        label: stateLabel('Headlights', controls.headlightsEnabled),
        disabled: !actionable || controls.headlightsEnabled === null,
      }
    );
  }
  if (controls.fatalErrorClearAvailable && controls.fatalErrorLatched === true) {
    actions.push({
      kind: 'clearFatalError',
      label: 'Clear fatal error',
      disabled: !actionable || !idle || controls.fatalErrorActive !== false || controls.fatalErrorClearable !== true,
    });
  }
  return actions;
}

export function shouldConfirmDisconnect(rideState: RideState | null | undefined): boolean {
  return rideState !== 'idle';
}

export function readingsAreDimmed(connection: ActiveConnection): boolean {
  return connection.state !== 'ready';
}

export function controllerRouteDescription(route: ControllerConnection | null): string {
  if (route?.kind === 'can') return `CAN ${route.canId}`;
  if (route?.kind === 'direct') return 'Direct Bluetooth';
  return 'Detecting…';
}

export function firmwareAccessDescription(firmware: FirmwareInfo | null): string {
  if (!firmware) return '—';
  return firmware.accessMode === 'readOnly' ? 'Read-only (test firmware)' : 'Read/write';
}

export function sortedControllerCandidates(candidates: readonly ControllerCandidate[]): ControllerCandidate[] {
  return [...candidates].sort((left, right) => left.canId - right.canId);
}
