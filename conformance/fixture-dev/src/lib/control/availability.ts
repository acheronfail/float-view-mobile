import type { FixtureSnapshot } from '../contract.ts';

export function controlAvailability(snapshot: FixtureSnapshot) {
	const attached = snapshot.serial.status === 'attached';
	const active =
		snapshot.power === 'on' ||
		snapshot.ble === 'advertising' ||
		snapshot.ble === 'connected';
	return {
		configure: !active,
		attach: snapshot.serial.status === 'detached',
		detach: attached,
		powerOn: attached && snapshot.power === 'off',
		powerOff: attached && snapshot.power === 'on',
		startRide: attached && snapshot.power === 'on' && snapshot.ride === 'idle',
		stopRide: attached && snapshot.power === 'on' && snapshot.ride === 'riding',
		directTelemetryUnavailable: snapshot.configuration.topology === 'direct'
	};
}

export function canonicalCanId(value: string): number | undefined {
	if (!/^(0|[1-9][0-9]{0,2})$/.test(value)) return undefined;
	const parsed = Number(value);
	return parsed <= 253 ? parsed : undefined;
}

export function shouldReplaceSnapshot(current: FixtureSnapshot, next: FixtureSnapshot): boolean {
	return next.revision >= current.revision;
}

export function startBoundedPolling(
	poll: () => void,
	intervalMs: number,
	schedule: typeof setInterval = setInterval,
	cancel: typeof clearInterval = clearInterval
) {
	const timer = schedule(poll, intervalMs);
	return () => cancel(timer);
}
