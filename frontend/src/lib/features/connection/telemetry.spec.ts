import { describe, expect, it } from 'vitest';
import {
	extensionTelemetry,
	currentTelemetryFields,
	slowTelemetryAge,
	telemetryFlag,
	telemetryState
} from './telemetry';

describe('telemetry presentation', () => {
	it('renders unavailable values explicitly and humanizes stable states', () => {
		expect(telemetryState(null)).toBe('Unavailable');
		expect(telemetryState('switchHalf')).toBe('Switch Half');
		expect(telemetryFlag(undefined)).toBe('Unavailable');
		expect(telemetryFlag(true)).toBe('Active');
		expect(telemetryFlag(false)).toBe('Inactive');
	});

	it('reports the age of compact telemetry without using wall-clock time', () => {
		expect(
			slowTelemetryAge({
				observedAt: '2026-07-30T10:00:02.500Z',
				slowObservedAt: '2026-07-30T10:00:00.000Z'
			})
		).toBe('2.5 s old');
		expect(slowTelemetryAge({ observedAt: 'invalid', slowObservedAt: null })).toBe('Unavailable');
	});

	it('sorts bounded negotiated extensions and retains raw numeric values', () => {
		const sample = {
			extensions: { 'future.z': 7, 'future.a': -1.25 }
		};
		expect(extensionTelemetry(sample)).toEqual([
			['future.a', '-1.25'],
			['future.z', '7']
		]);
	});

	it('keeps every raw current source distinct without utilization', () => {
		expect(currentTelemetryFields).toEqual([
			{ key: 'motorCurrentA', label: 'Actual motor current' },
			{ key: 'directionalMotorCurrentA', label: 'Directional motor current' },
			{ key: 'filteredMotorCurrentA', label: 'Filtered motor current' },
			{ key: 'batteryCurrentA', label: 'Battery current' },
			{ key: 'requestedBalanceCurrentA', label: 'Requested balance current' },
			{ key: 'boosterCurrentA', label: 'Booster current' },
			{ key: 'fieldWeakeningCurrentA', label: 'Field-weakening current' }
		]);
		expect(currentTelemetryFields.some((field) => field.label.includes('utilization'))).toBe(false);
	});
});
