import type {
	BoardConfiguration,
	ComposedBoardConfiguration,
	ControllerConfiguration
} from './boards.ts';
import { encodeFrame, FrameAssembler } from './frame.ts';
import { idleTelemetry } from './telemetry.ts';

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
	private readonly board: BoardConfiguration | ComposedBoardConfiguration;
	private readonly now: () => number;
	private readonly frames = new FrameAssembler();
	private telemetryIndex = 0;
	readonly requests: RecordedRequest[] = [];

	readonly diagnostics: string[] = [];

	constructor(
		board: BoardConfiguration | ComposedBoardConfiguration,
		now: () => number = () => performance.now()
	) {
		this.board = board;
		this.now = now;
	}

	receive(bytes: Uint8Array): SimulatedResponse[] {
		const responses: SimulatedResponse[] = [];
		for (const payload of this.frames.append(bytes)) {
			const value = this.response(payload);
			if (!value) continue;
			this.requests.push(value.request);
			responses.push({ frame: encodeFrame(value.response), request: value.request });
		}
		return responses;
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

	private response(
		payload: Uint8Array
	): { response: Uint8Array; request: RecordedRequest } | undefined {
		if ('kind' in this.board) {
			return this.composedResponse(payload);
		}
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

	private composedResponse(
		payload: Uint8Array
	): { response: Uint8Array; request: RecordedRequest } | undefined {
		const board = this.board as ComposedBoardConfiguration;
		const command = payload[0];
		if (command === undefined) return this.reject('Empty VESC request');
		const at = this.now();
		if (board.topology.kind === 'direct') {
			if (command === 0) {
				return {
					response: board.firmware.payload,
					request: { at, command, kind: 'controller-firmware' }
				};
			}
			return this.composedControllerResponse(board, payload, at);
		}
		if (command === 0) {
			return {
				response: board.expressTransport.payload,
				request: { at, command, kind: 'transport-firmware' }
			};
		}
		if (command === 62) {
			return {
				response: Uint8Array.of(62, board.topology.canId),
				request: { at, command, kind: 'can-ping' }
			};
		}
		if (command !== 34 || payload.length < 3) {
			return this.reject(`Unsafe or unsupported VESC command ${command}`);
		}
		if (payload[1] !== board.topology.canId) {
			return this.reject(`Unknown CAN target ${String(payload[1])}`);
		}
		return this.composedControllerResponse(board, payload.slice(2), at, board.topology.canId);
	}

	private composedControllerResponse(
		board: ComposedBoardConfiguration,
		payload: Uint8Array,
		at: number,
		canId?: number
	): { response: Uint8Array; request: RecordedRequest } | undefined {
		const command = payload[0];
		if (command === 0) {
			return {
				response: board.firmware.payload,
				request: { at, command, canId, kind: 'controller-firmware' }
			};
		}
		if (command !== 36 || payload[1] !== 101) {
			return this.reject(`Unsafe or unsupported controller command ${String(command)}`);
		}
		const packet = payload[2];
		const isTelemetry = packet === 1 || packet === 10 || packet === 31;
		const phase = board.telemetry.phase();
		const response = board.refloat.respond(payload, isTelemetry ? board.telemetry.next() : idleTelemetry);
		if (!response) return this.reject(`Unsupported Refloat request ${String(payload[2])}`);
		const kind =
			response.kind === 'info'
				? 'refloat-info'
				: response.kind === 'layout'
					? 'refloat-layout'
					: 'telemetry';
		return {
			response: response.payload,
			request: {
				at,
				command,
				canId,
				kind,
				...(kind === 'telemetry' ? { telemetryPhase: phase } : {})
			}
		};
	}

	private reject(message: string): undefined {
		const board = this.board;
		if (!('kind' in board) || board.requestPolicy === 'strict') throw new Error(message);
		if (this.diagnostics.length < 100) this.diagnostics.push(message);
		return undefined;
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
