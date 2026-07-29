import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { FrameAssembler } from './frame.ts';

export interface FirmwareIdentity {
	major: number;
	minor: number;
	hardwareName: string;
	hardwareType: number;
	payload: Uint8Array;
}

export interface TelemetryPhase {
	name: 'idle' | 'riding' | 'footpad-warning' | 'post-ride-tail' | 'return-to-idle';
	repetitions: number;
	payload: Uint8Array;
}

export interface ControllerConfiguration {
	canId: number;
	firmware: FirmwareIdentity;
	refloatVersion: string;
	refloatInfoPayload: Uint8Array;
	telemetryLayoutPayload: Uint8Array;
	telemetryPhases: TelemetryPhase[];
}

export interface BoardConfiguration {
	name: string;
	advertisedName: string;
	transport: FirmwareIdentity;
	controllers: ControllerConfiguration[];
}

const expressFirmware = fixturePayload('../backends/fixtures/vesc/firmware-express-6.05.json');
const controllerFirmware = fixturePayload('../backends/fixtures/vesc/firmware-6.06.json');
const refloat = fixtureCases('../backends/fixtures/refloat/rtdata-1.2.3-android-hardware.json');
const rideStates = fixtureCases('../backends/fixtures/refloat/ride-states-1.2.3.json');

export const expressThor: BoardConfiguration = {
	name: 'Express / Thor300 / Refloat',
	advertisedName: 'FloatView Test VESC',
	transport: {
		major: 6,
		minor: 5,
		hardwareName: 'VESC Express T',
		hardwareType: 2,
		payload: expressFirmware
	},
	controllers: [
		{
			canId: 1,
			firmware: {
				major: 6,
				minor: 6,
				hardwareName: 'THOR300',
				hardwareType: 0,
				payload: controllerFirmware
			},
			refloatVersion: '1.2.3',
			refloatInfoPayload: required(refloat.get('versioned-info-v2'), 'Refloat info fixture'),
			telemetryLayoutPayload: required(
				refloat.get('dynamic-realtime-field-ids'),
				'Refloat telemetry layout fixture'
			),
			telemetryPhases: [
				phase('idle', 4),
				phase('riding', 12),
				phase('footpad-warning', 2),
				phase('post-ride-tail', 4),
				phase('return-to-idle', 2)
			]
		}
	]
};

function phase(name: TelemetryPhase['name'], repetitions: number): TelemetryPhase {
	return {
		name,
		repetitions,
		payload: required(rideStates.get(name), `Refloat ${name} fixture`)
	};
}

function fixturePayload(relativePath: string): Uint8Array {
	const value = fixtureJson(relativePath) as { frameHex?: string };
	if (!value.frameHex) throw new Error(`Fixture ${relativePath} has no frameHex`);
	return decodeFrame(value.frameHex);
}

function fixtureCases(relativePath: string): Map<string, Uint8Array> {
	const value = fixtureJson(relativePath) as {
		cases?: Array<{ name?: string; frameHex?: string }>;
	};
	return new Map(
		(value.cases ?? []).map((item) => {
			if (!item.name || !item.frameHex) throw new Error(`Invalid fixture case in ${relativePath}`);
			return [item.name, decodeFrame(item.frameHex)];
		})
	);
}

function fixtureJson(relativePath: string): unknown {
	const path = fileURLToPath(new URL(relativePath, new URL('../', import.meta.url)));
	return JSON.parse(readFileSync(path, 'utf8'));
}

function decodeFrame(hex: string): Uint8Array {
	const frames = new FrameAssembler().append(Uint8Array.from(Buffer.from(hex, 'hex')));
	if (frames.length !== 1 || !frames[0]) throw new Error('Fixture must contain exactly one VESC frame');
	return frames[0];
}

function required<T>(value: T | undefined, name: string): T {
	if (value === undefined) throw new Error(`${name} is missing`);
	return value;
}
