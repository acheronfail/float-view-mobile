import type { ControllerCurrentLimits, LiveSample } from '../../backend/contract';

type GaugeSample = Pick<
	LiveSample,
	| 'dutyCycle'
	| 'motorCurrentA'
	| 'directionalMotorCurrentA'
	| 'filteredMotorCurrentA'
	| 'batteryCurrentA'
>;

const percentage = (value: number, limit: number) =>
	Math.min(Math.abs(value) / Math.abs(limit), 1) * 100;

export function driveLoadPercent(
	sample: GaugeSample | null | undefined,
	limits: ControllerCurrentLimits | null | undefined
): number | null {
	if (!sample) return null;

	const duty =
		sample.dutyCycle == null ? null : Math.min(Math.abs(sample.dutyCycle), 1) * 100;
	const motorCurrent =
		sample.filteredMotorCurrentA ?? sample.directionalMotorCurrentA ?? sample.motorCurrentA;
	const motorLimit =
		sample.motorCurrentA == null || !limits
			? null
			: sample.motorCurrentA < 0
				? limits.motorCurrentMinA
				: limits.motorCurrentMaxA;
	const motor =
		motorCurrent == null || motorLimit == null || motorLimit === 0
			? null
			: percentage(motorCurrent, motorLimit);
	const batteryLimit =
		sample.batteryCurrentA == null || !limits
			? null
			: sample.batteryCurrentA < 0
				? limits.batteryCurrentMinA
				: limits.batteryCurrentMaxA;
	const battery =
		sample.batteryCurrentA == null || batteryLimit == null || batteryLimit === 0
			? null
			: percentage(sample.batteryCurrentA, batteryLimit);
	const values = [duty, motor, battery].filter((value): value is number => value != null);
	return values.length === 0 ? null : Math.max(...values);
}
