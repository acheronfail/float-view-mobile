import type { ActiveConnection, RideState } from '$lib/backend/contract';

export function shouldConfirmDisconnect(rideState: RideState | null | undefined): boolean {
	return rideState !== 'idle';
}

export function readingsAreDimmed(connection: ActiveConnection): boolean {
	return connection.state !== 'ready';
}

export function isPrimaryPointerActivation(
	event: Pick<PointerEvent, 'button' | 'isPrimary'>
): boolean {
	return event.isPrimary && event.button === 0;
}

export function isKeyboardOrAssistiveClick(event: Pick<MouseEvent, 'detail'>): boolean {
	return event.detail === 0;
}
