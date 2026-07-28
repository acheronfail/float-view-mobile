import type { BoardConfiguration } from './boards.js';
import { encodeFrame, FrameAssembler } from './frame.js';

export class VescSimulator {
	private readonly frames = new FrameAssembler();
	readonly requests: Array<{ at: number; command: number }> = [];

	constructor(
		private readonly board: BoardConfiguration,
		private readonly now: () => number = Date.now
	) {}

	receive(bytes: Uint8Array): Uint8Array[] {
		return this.frames.append(bytes).map((payload) => {
			const response = this.response(payload);
			return encodeFrame(response);
		});
	}

	private response(payload: Uint8Array): Uint8Array {
		const command = payload[0];
		if (command === undefined) throw new Error('Empty VESC request');
		this.requests.push({ at: this.now(), command });
		switch (command) {
			case 0:
				return this.firmware(this.board.firmware);
			case 62:
				return Uint8Array.of(62, ...this.board.canIds);
			case 4:
				return this.realtime();
			case 36:
				return this.refloat(payload);
			case 34: {
				const canId = payload[1];
				if (canId === undefined || !this.board.canIds.includes(canId)) {
					throw new Error(`Unknown CAN target ${String(canId)}`);
				}
				const nested = this.response(payload.slice(2));
				return nested;
			}
			default:
				throw new Error(`Unsafe or unsupported VESC command ${command}`);
		}
	}

	private firmware(value: BoardConfiguration['firmware']) {
		return Uint8Array.of(
			0,
			value.major,
			value.minor,
			...Buffer.from(value.hardware),
			0,
			...new Uint8Array(20)
		);
	}

	private refloat(payload: Uint8Array) {
		if (payload[1] !== 101) throw new Error('Unknown custom application');
		if (payload[2] === 0) {
			return Uint8Array.of(36, 101, 0, ...Buffer.from(this.board.refloatVersion), 0);
		}
		if (payload[2] === 32) {
			return Uint8Array.of(36, 101, 32, ...Buffer.from('ride_state,footpad_state'), 0);
		}
		if (payload[2] === 31) {
			return Uint8Array.of(
				36,
				101,
				31,
				this.board.telemetry.rideState,
				this.board.telemetry.footpadState
			);
		}
		throw new Error(`Unsupported Refloat request ${String(payload[2])}`);
	}

	private realtime() {
		const bytes = new Uint8Array(30);
		bytes[0] = 4;
		writeI16(bytes, 1, 250);
		writeI16(bytes, 3, 240);
		writeI16(bytes, 17, Math.round(this.board.telemetry.dutyCycle * 1_000));
		writeI32(bytes, 19, this.board.telemetry.erpm);
		writeI16(bytes, 23, Math.round(this.board.telemetry.packVoltageV * 10));
		return bytes;
	}
}

function writeI16(bytes: Uint8Array, offset: number, value: number) {
	bytes[offset] = (value >> 8) & 0xff;
	bytes[offset + 1] = value & 0xff;
}

function writeI32(bytes: Uint8Array, offset: number, value: number) {
	bytes[offset] = (value >> 24) & 0xff;
	bytes[offset + 1] = (value >> 16) & 0xff;
	bytes[offset + 2] = (value >> 8) & 0xff;
	bytes[offset + 3] = value & 0xff;
}
