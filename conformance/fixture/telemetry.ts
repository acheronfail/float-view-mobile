export interface TelemetrySample {
	rideState: 'idle' | 'riding';
	boardState: number;
	footpadState: number;
	speedKph: number;
	erpm: number;
	dutyCycle: number;
	motorCurrentA: number;
	batteryCurrentA: number;
	packVoltageV: number;
	controllerTempC: number;
	motorTempC: number;
}

export interface TelemetrySource {
	next(): TelemetrySample;
	phase(): 'idle' | 'riding';
}

export class InteractiveTelemetrySource implements TelemetrySource {
	private riding = false;
	private index = 0;
	private readonly seed: number;
	private latest: TelemetrySample = idleTelemetry;

	constructor(seed = 0x5eed1234) {
		this.seed = seed >>> 0;
	}

	start(): boolean {
		if (this.riding) return false;
		this.riding = true;
		this.index = 0;
		return true;
	}

	stop(): boolean {
		if (!this.riding) return false;
		this.riding = false;
		return true;
	}

	phase() {
		return this.riding ? 'riding' : 'idle';
	}

	sampleIndex() {
		return this.index;
	}

	next(): TelemetrySample {
		if (!this.riding) {
			this.latest = idleTelemetry;
			return this.latest;
		}
		const index = this.index++;
		const slow = Math.sin(index / 8);
		const load = 0.58 + Math.sin(index / 4) * 0.08 + jitter(this.seed, index) * 0.025;
		const speedKph = 21 + slow * 2.2 + jitter(this.seed ^ 0x91e10da5, index) * 0.35;
		const erpm = Math.round(speedKph * 235);
		const dutyCycle = clamp(load, 0.35, 0.8);
		const motorCurrentA = 18 + dutyCycle * 22;
		const batteryCurrentA = motorCurrentA * dutyCycle * 0.82;
		this.latest = {
			rideState: 'riding',
			boardState: 3,
			footpadState: 3,
			speedKph,
			erpm,
			dutyCycle,
			motorCurrentA,
			batteryCurrentA,
			packVoltageV: 63.2 - batteryCurrentA * 0.035,
			controllerTempC: 31 + Math.min(index, 300) * 0.015 + slow * 0.15,
			motorTempC: 29 + Math.min(index, 300) * 0.012 + slow * 0.12
		};
		return this.latest;
	}

	lastSample() {
		return this.latest;
	}
}

export const idleTelemetry: TelemetrySample = {
	rideState: 'idle',
	boardState: 0,
	footpadState: 0,
	speedKph: 0,
	erpm: 0,
	dutyCycle: 0,
	motorCurrentA: 0,
	batteryCurrentA: 0,
	packVoltageV: 63.4,
	controllerTempC: 30,
	motorTempC: 28
};

function jitter(seed: number, index: number): number {
	let value = (seed + Math.imul(index + 1, 0x9e3779b1)) >>> 0;
	value ^= value << 13;
	value ^= value >>> 17;
	value ^= value << 5;
	return ((value >>> 0) / 0xffffffff) * 2 - 1;
}

function clamp(value: number, minimum: number, maximum: number) {
	return Math.max(minimum, Math.min(maximum, value));
}
