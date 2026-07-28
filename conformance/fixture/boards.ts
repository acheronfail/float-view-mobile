export interface BoardConfiguration {
	name: string;
	firmware: { major: number; minor: number; hardware: string };
	canIds: number[];
	refloatVersion: string;
	telemetry: {
		packVoltageV: number;
		dutyCycle: number;
		erpm: number;
		rideState: number;
		footpadState: number;
	};
}

export const expressThor: BoardConfiguration = {
	name: 'Express / Thor300 / Refloat',
	firmware: { major: 6, minor: 6, hardware: 'EXPRESS' },
	canIds: [1],
	refloatVersion: '1.2.3',
	telemetry: {
		packVoltageV: 75.6,
		dutyCycle: 0.42,
		erpm: 3_200,
		rideState: 2,
		footpadState: 3
	}
};
