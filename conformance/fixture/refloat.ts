import type { TelemetrySample } from './telemetry.ts';

export type RefloatVersion = '1.0.0' | '1.1.0' | '1.2.3';
export type RefloatRequestKind = 'info' | 'layout' | 'telemetry';

export interface RefloatResponse {
	kind: RefloatRequestKind;
	payload: Uint8Array;
}

export interface RefloatCodec {
	version: RefloatVersion;
	provenance: string;
	infoPayload: Uint8Array;
	layoutPayload?: Uint8Array;
	respond(payload: Uint8Array, sample: TelemetrySample): RefloatResponse | undefined;
}

export const realtimeFields = [
	'motor.speed',
	'motor.erpm',
	'motor.current',
	'motor.duty_cycle',
	'motor.batt_voltage',
	'motor.batt_current',
	'motor.mosfet_temp',
	'motor.motor_temp'
] as const;

const provenance =
	'Read-only protocol shapes derived from lukash/refloat tags v1.0.0, v1.1.0, and v1.2.3';

export const refloatCodecs: Readonly<Record<RefloatVersion, RefloatCodec>> = {
	'1.0.0': legacyCodec(),
	'1.1.0': dynamicCodec('1.1.0', false),
	'1.2.3': dynamicCodec('1.2.3', true)
};

export function encodeRefloatTelemetry(
	version: RefloatVersion,
	sample: TelemetrySample
): Uint8Array {
	return version === '1.0.0'
		? encodeLegacyTelemetry(sample, 10)
		: encodeDynamicTelemetry(sample, version === '1.2.3');
}

function legacyCodec(): RefloatCodec {
	const infoPayload = Uint8Array.of(36, 101, 0, 10);
	return {
		version: '1.0.0',
		provenance,
		infoPayload,
		respond(payload, sample) {
			const packet = packagePacket(payload);
			if (packet === 0) return { kind: 'info', payload: infoPayload };
			if (packet === 1 || packet === 10) {
				return { kind: 'telemetry', payload: encodeLegacyTelemetry(sample, packet) };
			}
			return undefined;
		}
	};
}

function dynamicCodec(version: '1.1.0' | '1.2.3', alertTail: boolean): RefloatCodec {
	const infoPayload = encodeInfoV2(version);
	const layoutPayload = encodeFieldLayout(realtimeFields);
	return {
		version,
		provenance,
		infoPayload,
		layoutPayload,
		respond(payload, sample) {
			const packet = packagePacket(payload);
			if (packet === 0) return { kind: 'info', payload: infoPayload };
			if (packet === 32) return { kind: 'layout', payload: layoutPayload };
			if (packet === 31) {
				return { kind: 'telemetry', payload: encodeDynamicTelemetry(sample, alertTail) };
			}
			return undefined;
		}
	};
}

function packagePacket(payload: Uint8Array): number | undefined {
	if (payload[0] !== 36 || payload[1] !== 101) return undefined;
	return payload[2];
}

function encodeInfoV2(version: '1.1.0' | '1.2.3'): Uint8Array {
	const payload = new Uint8Array(61);
	payload.set([36, 101, 0, 2]);
	payload.set(new TextEncoder().encode('Refloat').slice(0, 20), 5);
	const [major, minor, patch] = version.split('.').map(Number) as [number, number, number];
	payload.set([major, minor, patch], 25);
	return payload;
}

function encodeFieldLayout(fields: readonly string[]): Uint8Array {
	const bytes: number[] = [36, 101, 32, fields.length];
	for (const field of fields) {
		const encoded = new TextEncoder().encode(field);
		bytes.push(encoded.length, ...encoded);
	}
	bytes.push(0);
	return Uint8Array.from(bytes);
}

function encodeDynamicTelemetry(sample: TelemetrySample, alertTail: boolean): Uint8Array {
	const values = [
		sample.speedKph / 3.6,
		sample.erpm,
		sample.motorCurrentA,
		sample.dutyCycle,
		sample.packVoltageV,
		sample.batteryCurrentA,
		sample.controllerTempC,
		sample.motorTempC
	];
	const payload = new Uint8Array(13 + values.length * 2 + (alertTail ? 4 : 0));
	payload.set([36, 101, 31, 0]);
	payload[9] = sample.boardState & 0x03;
	payload[10] = (sample.footpadState & 0x03) << 6;
	let offset = 13;
	for (const value of values) {
		const bits = float16(value);
		payload[offset++] = bits >> 8;
		payload[offset++] = bits & 0xff;
	}
	if (alertTail) payload.set([0, 0, 0, 0], offset);
	return payload;
}

function encodeLegacyTelemetry(sample: TelemetrySample, packet: number): Uint8Array {
	const payload = new Uint8Array(55);
	payload.set([36, 101, packet, 3]);
	payload[10] = sample.rideState === 'riding' ? 1 : 0;
	payload[11] = sample.footpadState === 3 ? 2 : sample.footpadState ? 1 : 0;
	payload[12] = sample.footpadState === 1 ? 100 : sample.footpadState === 2 ? 20 : 0;
	payload[13] = sample.footpadState === 2 ? 100 : sample.footpadState === 1 ? 20 : 0;
	i16(payload, 23, sample.packVoltageV * 10);
	i16(payload, 25, sample.erpm);
	i16(payload, 27, (sample.speedKph / 3.6) * 10);
	i16(payload, 29, sample.motorCurrentA * 10);
	i16(payload, 31, sample.batteryCurrentA * 10);
	payload[33] = Math.round(128 + sample.dutyCycle * 100);
	payload[39] = Math.round(sample.controllerTempC * 2);
	payload[40] = Math.round(sample.motorTempC * 2);
	return payload;
}

function i16(destination: Uint8Array, offset: number, value: number) {
	const bounded = Math.max(-32768, Math.min(32767, Math.round(value)));
	destination[offset] = (bounded >> 8) & 0xff;
	destination[offset + 1] = bounded & 0xff;
}

export function float16(value: number): number {
	if (!Number.isFinite(value)) throw new Error('Float16 value must be finite');
	const buffer = new ArrayBuffer(4);
	const view = new DataView(buffer);
	view.setFloat32(0, value, false);
	const bits = view.getUint32(0, false);
	const sign = (bits >>> 16) & 0x8000;
	let exponent = ((bits >>> 23) & 0xff) - 127 + 15;
	let fraction = bits & 0x7fffff;
	if (exponent <= 0) {
		if (exponent < -10) return sign;
		fraction = (fraction | 0x800000) >>> (1 - exponent);
		return sign | ((fraction + 0x1000) >>> 13);
	}
	if (exponent >= 31) return sign | 0x7bff;
	fraction += 0x1000;
	if (fraction & 0x800000) {
		fraction = 0;
		exponent += 1;
	}
	return sign | (exponent << 10) | (fraction >>> 13);
}
