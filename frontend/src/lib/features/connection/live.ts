import type {
  ActiveConnection,
  ControllerCandidate,
  ControllerConnection,
  FirmwareInfo,
  RefloatConfigurationSummary,
  RideState,
  RefloatControl,
} from '$lib/backend/contract';

export type RefloatMenuAction = {
  kind: 'control';
  control: 'package' | 'flywheel';
  enabled: boolean;
  label: string;
  disabled: boolean;
  pending: boolean;
};

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

export function refloatConfigurationWriteUnavailableReason(
  connection: ActiveConnection | null | undefined,
  summary: RefloatConfigurationSummary | null | undefined
): string | null {
  if (!connection || connection.state !== 'ready') return 'Reconnect to the board before applying a Tune.';
  if (summary?.availability !== 'ready' || !summary.writable) {
    return 'Persistent Refloat writes are currently unavailable.';
  }
  if (
    summary.pendingOperation !== null ||
    connection.refloatControls?.pendingAction != null ||
    connection.controllerActions.pendingAction !== null
  ) {
    return 'Wait for the current board operation to finish before applying a Tune.';
  }
  if (connection.refloatControls?.flywheelEnabled === true) return 'Exit flywheel mode before applying a Tune.';

  const sample = connection.latestSample;
  if (!sample) return 'Wait for live board status before applying a Tune.';
  if (sample.rideState !== 'idle') return 'Disengage the board before applying a Tune.';
  if (sample.footpadState !== 'none') return 'Step off both footpads before applying a Tune.';
  if (sample.packageState !== 'ready' || sample.packageMode !== 'normal') {
    return 'Wait until Refloat is ready in normal mode before applying a Tune.';
  }
  return null;
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
      description: 'Balancing will be disabled. Keep the board disengaged before continuing.',
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

/** Explain why fault history cannot currently offer a clear action. */
export function fatalErrorClearUnavailableReason(connection: ActiveConnection | null | undefined): string | null {
  const controls = connection?.refloatControls;
  if (connection?.state !== 'ready') return 'Reconnect to the board to clear the latch.';
  if (!controls || controls.status === 'unavailable') return 'Wait for live board status.';
  if (!controls.fatalErrorClearAvailable)
    return 'This Refloat version cannot clear the latch from the app. After checking the fault log, power cycle the board.';
  if (!controls.writable) return 'This connection is read-only. The latch cannot be cleared from the app.';
  if (controls.flywheelEnabled) return 'Exit flywheel mode before clearing the latch.';
  if (controls.fatalErrorActive === true || connection.latestSample?.firmwareFault)
    return 'The controller fault is still active. It must end before the latch can be cleared.';
  if (connection.latestSample?.rideState !== 'idle') return 'Step off the board and wait until it is idle.';
  if (controls.pendingAction !== null) return 'Wait for the current board operation to finish.';
  if (
    controls.status !== 'ready' ||
    controls.fatalErrorLatched !== true ||
    controls.fatalErrorActive !== false ||
    controls.fatalErrorClearable !== true
  )
    return 'Waiting for the board to confirm that the latch can be cleared.';
  return null;
}

export function refloatMenuActions(connection: ActiveConnection): RefloatMenuAction[] {
  const controls = connection.refloatControls;
  if (!controls || controls.status === 'unavailable') return [];
  const actionable =
    connection.state === 'ready' && controls.status === 'ready' && controls.writable && controls.pendingAction === null;
  const idle = connection.latestSample?.rideState === 'idle';
  const flywheel = controls.flywheelEnabled === true;

  if (flywheel) return [];

  const actions: RefloatMenuAction[] = [
    {
      kind: 'control',
      control: 'package',
      enabled: !(controls.packageEnabled ?? true),
      label:
        controls.packageEnabled === null
          ? 'Refloat (Unknown)'
          : controls.packageEnabled
            ? 'Disable Refloat…'
            : 'Enable Refloat',
      pending: controls.pendingAction === 'package',
      disabled: !actionable || !idle || controls.packageEnabled === null,
    },
    {
      kind: 'control',
      control: 'flywheel',
      enabled: !(controls.flywheelEnabled ?? false),
      label: controls.flywheelEnabled === null ? 'Flywheel (Unknown)' : 'Enable flywheel…',
      pending: controls.pendingAction === 'flywheel',
      disabled:
        !actionable ||
        !idle ||
        controls.flywheelEnabled === null ||
        (controls.flywheelEnabled === false && connection.latestSample?.packageState !== 'ready'),
    },
  ];
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
