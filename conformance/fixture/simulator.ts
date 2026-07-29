import type { BoardConfiguration, ControllerConfiguration } from './boards.ts';
import { encodeFrame, FrameAssembler } from './frame.ts';

export type VescRequestKind =
	| 'transport-firmware'
	| 'can-ping'
	| 'controller-firmware'
	| 'refloat-info'
	| 'refloat-layout'
	| 'telemetry';

export interface RecordedRequest {
	at: number;
	command: number;
	kind: VescRequestKind;
	canId?: number;
	telemetryPhase?: string;
}

export interface SimulatedResponse {
	frame: Uint8Array;
	request: RecordedRequest;
}

export class VescSimulator {
	private readonly board: BoardConfiguration;
	private readonly now: () => number;
	private readonly frames = new FrameAssembler();
	private telemetryIndex = 0;
	readonly requests: RecordedRequest[] = [];

	constructor(board: BoardConfiguration, now: () => number = () => performance.now()) {
		this.board = board;
		this.now = now;
	}

	receive(bytes: Uint8Array): SimulatedResponse[] {
		return this.frames.append(bytes).map((payload) => {
			const { response, request } = this.response(payload);
			this.requests.push(request);
			return { frame: encodeFrame(response), request };
		});
	}

	verifyCadence(expectedIntervalMs: number, toleranceMs: number, minimumIntervals = 2) {
		const telemetry = this.requests.filter((request) => request.kind === 'telemetry');
		const intervals: number[] = [];
		for (let index = 1; index < telemetry.length; index += 1) {
			const previous = telemetry[index - 1]!;
			const current = telemetry[index]!;
			if (
				previous.telemetryPhase === 'idle' ||
				current.telemetryPhase === 'idle' ||
				current.telemetryPhase === 'return-to-idle'
			) {
				continue;
			}
			intervals.push(current.at - previous.at);
		}
		if (intervals.length < minimumIntervals) {
			throw new Error(
				`Telemetry cadence requires ${minimumIntervals} riding intervals, observed ${intervals.length}`
			);
		}
		for (const interval of intervals) {
			if (Math.abs(interval - expectedIntervalMs) > toleranceMs) {
				throw new Error(
					`Telemetry cadence expected ${expectedIntervalMs}±${toleranceMs}ms, observed ${interval}ms`
				);
			}
		}
	}

	resetTelemetry() {
		this.telemetryIndex = 0;
	}

	private response(payload: Uint8Array): { response: Uint8Array; request: RecordedRequest } {
		const command = payload[0];
		if (command === undefined) throw new Error('Empty VESC request');
		const at = this.now();
		if (command === 0) {
			return {
				response: this.board.transport.payload,
				request: { at, command, kind: 'transport-firmware' }
			};
		}
		if (command === 62) {
			return {
				response: Uint8Array.of(62, ...this.board.controllers.map((item) => item.canId)),
				request: { at, command, kind: 'can-ping' }
			};
		}
		if (command !== 34) {
			throw new Error(`Unsafe or unsupported VESC command ${command}`);
		}
		const canId = payload[1];
		const nestedCommand = payload[2];
		if (canId === undefined || nestedCommand === undefined) {
			throw new Error('Truncated CAN-forward request');
		}
		const controller = this.board.controllers.find((item) => item.canId === canId);
		if (!controller) throw new Error(`Unknown CAN target ${canId}`);
		return this.controllerResponse(controller, payload.slice(2), at);
	}

	private controllerResponse(
		controller: ControllerConfiguration,
		payload: Uint8Array,
		at: number
	): { response: Uint8Array; request: RecordedRequest } {
		const command = payload[0]!;
		const base = { at, command, canId: controller.canId };
		if (command === 0) {
			return {
				response: controller.firmware.payload,
				request: { ...base, kind: 'controller-firmware' }
			};
		}
		if (command !== 36 || payload[1] !== 101) {
			throw new Error(`Unsafe or unsupported forwarded command ${command}`);
		}
		switch (payload[2]) {
			case 0:
				return {
					response: controller.refloatInfoPayload,
					request: { ...base, kind: 'refloat-info' }
				};
			case 32:
				return {
					response: controller.telemetryLayoutPayload,
					request: { ...base, kind: 'refloat-layout' }
				};
			case 31: {
				const phase = telemetryPhase(controller, this.telemetryIndex++);
				return {
					response: phase.payload,
					request: { ...base, kind: 'telemetry', telemetryPhase: phase.name }
				};
			}
			default:
				throw new Error(`Unsupported Refloat request ${String(payload[2])}`);
		}
	}
}

function telemetryPhase(controller: ControllerConfiguration, index: number) {
	let remaining = index;
	for (const phase of controller.telemetryPhases) {
		if (remaining < phase.repetitions) return phase;
		remaining -= phase.repetitions;
	}
	return controller.telemetryPhases.at(-1) ??
		(() => {
			throw new Error('Controller has no telemetry phases');
		})();
}
