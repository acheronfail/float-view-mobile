import type {
	ActiveConnection,
	ControllerCandidate,
	ControllerConnection,
	FirmwareInfo,
	RideState
} from '$lib/backend/contract';

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

export function sortedControllerCandidates(
	candidates: readonly ControllerCandidate[]
): ControllerCandidate[] {
	return [...candidates].sort((left, right) => left.canId - right.canId);
}
