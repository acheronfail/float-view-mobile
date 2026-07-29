import type {
	ActiveConnection,
	ControllerCandidate,
	ControllerConnection,
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

export function sortedControllerCandidates(
	candidates: readonly ControllerCandidate[]
): ControllerCandidate[] {
	return [...candidates].sort((left, right) => left.canId - right.canId);
}
