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

export type DriveLoadSource = 'duty' | 'motor' | 'battery';

export interface DriveLoad {
	source: DriveLoadSource;
	percentage: number;
}

export function driveLoad(
	sample: GaugeSample | null | undefined,
	limits: ControllerCurrentLimits | null | undefined
): DriveLoad | null {
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
	const candidates: DriveLoad[] = [
		duty == null ? null : { source: 'duty', percentage: duty },
		motor == null ? null : { source: 'motor', percentage: motor },
		battery == null ? null : { source: 'battery', percentage: battery }
	].filter((value): value is DriveLoad => value != null);
	return (
		candidates.reduce<DriveLoad | null>(
			(winner, candidate) =>
				winner == null || candidate.percentage > winner.percentage ? candidate : winner,
			null
		) ?? null
	);
}

export function driveLoadPercent(
	sample: GaugeSample | null | undefined,
	limits: ControllerCurrentLimits | null | undefined
): number | null {
	return driveLoad(sample, limits)?.percentage ?? null;
}
