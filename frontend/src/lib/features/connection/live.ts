import type { ActiveConnection, RideState } from '$lib/backend/contract';

export function shouldConfirmDisconnect(rideState: RideState | null | undefined): boolean {
	return rideState !== 'idle';
}

export function readingsAreDimmed(connection: ActiveConnection): boolean {
	return connection.state !== 'ready';
}
