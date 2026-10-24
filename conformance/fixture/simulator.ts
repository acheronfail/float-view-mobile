import type { BoardConfiguration, ComposedBoardConfiguration, ControllerConfiguration } from './boards.ts';
import { encodeFrame, FrameAssembler } from './frame.ts';
import { motorConfigurationPayload } from './firmware.ts';
import { idleTelemetry } from './telemetry.ts';
import { encodeLegacyCompact } from './refloat.ts';
import { encodeBmsValuesPayload, type BmsConfiguration } from './bms.ts';

export type VescRequestKind =
  | 'transport-firmware'
  | 'can-ping'
  | 'controller-firmware'
  | 'controller-route-verification'
  | 'bms-firmware'
  | 'bms-values'
  | 'motor-configuration'
  | 'refloat-info'
  | 'refloat-layout'
  | 'refloat-control'
  | 'refloat-configuration-schema'
  | 'refloat-configuration-current'
  | 'refloat-configuration-default'
  | 'refloat-configuration-write'
  | 'controller-shutdown'
  | 'telemetry';

export interface RecordedRequest {
  at: number;
  command: number;
  kind: VescRequestKind;
  canId?: number;
  telemetryPhase?: string;
  telemetryCommand?: 'legacy-realtime' | 'realtime' | 'compact' | 'alerts';
}

export interface FixtureRequestAttempt {
  command: number;
  canId?: number;
  telemetryCommand?: 'legacy-realtime' | 'realtime' | 'compact' | 'alerts';
}

export interface SimulatedResponse {
  frame: Uint8Array;
  request: RecordedRequest;
}

type SimulatedTransaction = { response?: Uint8Array; request: RecordedRequest };

export class VescSimulator {
  private readonly board: BoardConfiguration | ComposedBoardConfiguration;
  private readonly now: () => number;
  private readonly frames = new FrameAssembler();
  private telemetryIndex = 0;
  private transportFirmwareAttempts = 0;
  readonly requests: RecordedRequest[] = [];

  readonly diagnostics: string[] = [];

  constructor(board: BoardConfiguration | ComposedBoardConfiguration, now: () => number = () => performance.now()) {
    this.board = board;
    this.now = now;
  }

  receive(bytes: Uint8Array, onAttempt?: (attempt: FixtureRequestAttempt) => void): SimulatedResponse[] {
    const responses: SimulatedResponse[] = [];
    for (const payload of this.frames.append(bytes)) {
      onAttempt?.(requestAttempt(payload));
      const value = this.response(payload);
      if (!value) continue;
      this.requests.push(value.request);
      if (value.response) responses.push({ frame: encodeFrame(value.response), request: value.request });
    }
    return responses;
  }

  verifyCadence(expectedIntervalMs: number, toleranceMs: number, minimumIntervals = 2) {
    const telemetry = this.requests.filter(
      (request) =>
        request.kind === 'telemetry' && request.telemetryCommand !== 'compact' && request.telemetryCommand !== 'alerts'
    );
    const intervals: number[] = [];
    for (let index = 1; index < telemetry.length; index += 1) {
      const previous = telemetry[index - 1]!;
      const current = telemetry[index]!;
      if (
        previous.telemetryPhase === 'idle' ||
        current.telemetryPhase === 'idle' ||
        current.telemetryPhase === 'return-to-idle' ||
        previous.telemetryPhase !== current.telemetryPhase
      ) {
        continue;
      }
      intervals.push(current.at - previous.at);
    }
    if (intervals.length < minimumIntervals) {
      throw new Error(`Telemetry cadence requires ${minimumIntervals} riding intervals, observed ${intervals.length}`);
    }
    for (const interval of intervals) {
      if (Math.abs(interval - expectedIntervalMs) > toleranceMs) {
        throw new Error(`Telemetry cadence expected ${expectedIntervalMs}±${toleranceMs}ms, observed ${interval}ms`);
      }
    }
  }

  resetTelemetry() {
    this.telemetryIndex = 0;
  }

  reset() {
    this.telemetryIndex = 0;
    if ('kind' in this.board) this.board.refloatConfiguration.reset();
    else for (const controller of this.board.controllers) controller.refloatConfiguration?.reset();
  }

  configurationSnapshots() {
    if ('kind' in this.board) {
      return [
        {
          route:
            this.board.topology.kind === 'direct' ? ('direct' as const) : (`CAN:${this.board.topology.canId}` as const),
          ...this.board.refloatConfiguration.snapshot(),
        },
      ];
    }
    const board = this.board;
    return board.controllers.flatMap((controller) =>
      controller.refloatConfiguration
        ? [
            {
              route: board.transport.hardwareType === 0 ? ('direct' as const) : (`CAN:${controller.canId}` as const),
              ...controller.refloatConfiguration.snapshot(),
            },
          ]
        : []
    );
  }

  private response(payload: Uint8Array): SimulatedTransaction | undefined {
    if ('kind' in this.board) {
      return this.composedResponse(payload);
    }
    const command = payload[0];
    if (command === undefined) throw new Error('Empty VESC request');
    const at = this.now();
    if (command === 0) {
      this.transportFirmwareAttempts += 1;
      return {
        response:
          this.board.dropFirstTransportFirmwareResponse && this.transportFirmwareAttempts === 1
            ? undefined
            : this.board.transport.payload,
        request: { at, command, kind: 'transport-firmware' },
      };
    }
    if (command === 62) {
      const defaultIds = [
        ...(this.board.transport.hardwareType === 0 ? [] : this.board.controllers.map((item) => item.canId)),
        ...(this.board.bmsNodes ?? [])
          .filter((item) => item.pingVisible !== false)
          .flatMap((item) => (item.canId === undefined ? [] : [item.canId])),
      ];
      return {
        response: Uint8Array.of(62, ...(this.board.canPingIds ?? defaultIds)),
        request: { at, command, kind: 'can-ping' },
      };
    }
    if (command === 96 && this.board.localBms) {
      return this.bmsResponse(this.board.localBms, payload, at);
    }
    if (command !== 34 && this.board.transport.hardwareType === 0) {
      const controller = this.board.controllers[0];
      if (!controller) throw new Error('Direct board has no controller configuration');
      return this.controllerResponse(controller, payload, at);
    }
    if (command !== 34) {
      throw new Error(`Unsafe or unsupported VESC command ${command}`);
    }
    const canId = payload[1];
    const nestedCommand = payload[2];
    if (canId === undefined || nestedCommand === undefined) {
      throw new Error('Truncated CAN-forward request');
    }
    if (this.board.silentCanIds?.includes(canId)) {
      return this.rejectBoard(`Silent CAN target ${canId}`);
    }
    const controller = this.board.controllers.find((item) => item.canId === canId);
    if (controller) return this.controllerResponse(controller, payload.slice(2), at, canId);
    const bms = this.board.bmsNodes?.find((item) => item.canId === canId);
    if (bms) return this.bmsResponse(bms, payload.slice(2), at, canId);
    return this.rejectBoard(`Unknown CAN target ${canId}`);
  }

  private composedResponse(payload: Uint8Array): SimulatedTransaction | undefined {
    const board = this.board as ComposedBoardConfiguration;
    const command = payload[0];
    if (command === undefined) return this.reject('Empty VESC request');
    const at = this.now();
    if (board.topology.kind === 'direct') {
      if (!board.shutdown.available) return undefined;
      if (command === 0) {
        return {
          response: board.firmware.payload,
          request: { at, command, kind: 'controller-firmware' },
        };
      }
      return this.composedControllerResponse(board, payload, at);
    }
    if (command === 0) {
      return {
        response: board.expressTransport.payload,
        request: { at, command, kind: 'transport-firmware' },
      };
    }
    if (command === 62) {
      return {
        response: Uint8Array.of(62, board.topology.canId),
        request: { at, command, kind: 'can-ping' },
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
  ): SimulatedTransaction | undefined {
    const command = payload[0];
    if (command === undefined) return this.reject('Empty controller request');
    if (!board.shutdown.available) return undefined;
    if (command === 0) {
      return {
        response: board.firmware.payload,
        request: { at, command, canId, kind: 'controller-firmware' },
      };
    }
    if (command === 50) {
      if (!matchesControllerCanIdRequest(payload)) return this.reject('Invalid controller CAN ID request');
      return {
        response: Uint8Array.of(50, 0, 2, 0, 0, canId ?? 0),
        request: { at, command, canId, kind: 'controller-route-verification' },
      };
    }
    if (command === 14) {
      return {
        response: motorConfigurationPayload,
        request: { at, command, canId, kind: 'motor-configuration' },
      };
    }
    if (command === 156) {
      const phase = board.telemetry.phase();
      const shutdown = board.shutdown.apply(board.firmware, payload, {
        rideState: phase === 'idle' ? 'idle' : 'riding',
        flywheel: board.refloatControls.state.flywheelEnabled,
        mutationPending: false,
      });
      if (!shutdown.accepted) return this.reject(`Rejected controller shutdown ${shutdown.reason}`);
      return { request: { at, command, canId, kind: 'controller-shutdown' } };
    }
    if (command >= 92 && command <= 95) {
      const safe = board.refloatControls.state.flywheelEnabled
        ? 'flywheel'
        : board.telemetry.phase() === 'idle'
          ? true
          : 'riding';
      const response = board.refloatConfiguration.respond(payload, safe);
      if (!response)
        return this.reject(
          `Rejected Refloat configuration command ${command}: ${board.refloatConfiguration.rejectionReason}`
        );
      const kind =
        command === 92
          ? 'refloat-configuration-schema'
          : command === 93
            ? 'refloat-configuration-current'
            : command === 94
              ? 'refloat-configuration-default'
              : 'refloat-configuration-write';
      return { response, request: { at, command, canId, kind } };
    }
    if (command !== 36 || payload[1] !== 101) {
      return this.reject(`Unsafe or unsupported controller command ${String(command)}`);
    }
    const packet = payload[2];
    if (packet !== undefined && [12, 20, 22, 36, 202].includes(packet)) {
      const phase = board.telemetry.phase();
      const control = board.refloatControls.apply(payload, {
        ...(canId === undefined ? {} : { selectedCanId: canId, requestCanId: canId }),
        firmwareTestVersion: board.firmware.testVersion,
        rideState: phase === 'idle' ? 'idle' : 'riding',
      });
      if (!control.accepted) return this.reject(`Rejected Refloat control ${String(control.reason)}`);
      return {
        response: control.response,
        request: { at, command, canId, kind: 'refloat-control' },
      };
    }
    if (packet === 35) {
      const response = board.refloatControls.alertsResponse(payload);
      if (!response) return this.reject('Unsupported Refloat alerts request');
      return { response, request: { at, command, canId, kind: 'telemetry', telemetryCommand: 'alerts' } };
    }
    const isTelemetry = packet === 1 || packet === 10 || packet === 31 || packet === 33;
    const phase = board.telemetry.phase();
    const sample = isTelemetry ? board.refloatControls.applyToSample(board.telemetry.next()) : idleTelemetry;
    const response = board.refloat.respond(payload, sample);
    if (!response) return this.reject(`Unsupported Refloat request ${String(payload[2])}`);
    const kind =
      response.kind === 'info' ? 'refloat-info' : response.kind === 'layout' ? 'refloat-layout' : 'telemetry';
    return {
      response: isTelemetry ? board.refloatControls.decorateTelemetry(response.payload) : response.payload,
      request: {
        at,
        command,
        canId,
        kind,
        ...(kind === 'telemetry'
          ? {
              telemetryPhase: phase,
              telemetryCommand: packet === 1 ? 'legacy-realtime' : packet === 10 ? 'compact' : 'realtime',
            }
          : {}),
      },
    };
  }

  private reject(message: string): undefined {
    const board = this.board;
    if (!('kind' in board) || board.requestPolicy === 'strict') throw new Error(message);
    if (this.diagnostics.length < 50) this.diagnostics.push(message);
    return undefined;
  }

  private rejectBoard(message: string): undefined {
    const board = this.board as BoardConfiguration;
    if (board.requestPolicy !== 'interactive') throw new Error(message);
    if (this.diagnostics.length < 50) this.diagnostics.push(message);
    return undefined;
  }

  private controllerResponse(
    controller: ControllerConfiguration,
    payload: Uint8Array,
    at: number,
    canId?: number
  ): SimulatedTransaction | undefined {
    const command = payload[0]!;
    const base = { at, command, ...(canId === undefined ? {} : { canId }) };
    if (controller.shutdown && !controller.shutdown.available) return undefined;
    if (command === 0) {
      return {
        response: controller.firmware.payload,
        request: { ...base, kind: 'controller-firmware' },
      };
    }
    if (command === 50) {
      if (!matchesControllerCanIdRequest(payload)) return this.rejectBoard('Invalid controller CAN ID request');
      return {
        response: Uint8Array.of(50, 0, 2, 0, 0, controller.canId),
        request: { ...base, kind: 'controller-route-verification' },
      };
    }
    if (command === 14) {
      return {
        response: motorConfigurationPayload,
        request: { ...base, kind: 'motor-configuration' },
      };
    }
    if (command === 156 && controller.shutdown) {
      const phase = controller.telemetry?.phase() ?? 'idle';
      const version = `${controller.firmware.major}.${controller.firmware.minor.toString().padStart(2, '0')}` as const;
      const shutdown = controller.shutdown.apply({ version, testVersion: 0 }, payload, {
        rideState: phase === 'idle' ? 'idle' : 'riding',
        flywheel: controller.refloatControls?.state.flywheelEnabled ?? false,
        mutationPending: false,
      });
      if (!shutdown.accepted) return this.rejectBoard(`Rejected controller shutdown ${shutdown.reason}`);
      return { request: { ...base, kind: 'controller-shutdown' } };
    }
    if (command >= 92 && command <= 95 && controller.refloatConfiguration) {
      const safe = controller.refloatControls?.state.flywheelEnabled
        ? 'flywheel'
        : (controller.telemetry?.phase() ?? 'idle') === 'idle'
          ? true
          : 'riding';
      const response = controller.refloatConfiguration.respond(payload, safe);
      if (!response)
        return this.rejectBoard(
          `Rejected Refloat configuration command ${command}: ${controller.refloatConfiguration.rejectionReason}`
        );
      const kind =
        command === 92
          ? 'refloat-configuration-schema'
          : command === 93
            ? 'refloat-configuration-current'
            : command === 94
              ? 'refloat-configuration-default'
              : 'refloat-configuration-write';
      return { response, request: { ...base, kind } };
    }
    if (command !== 36 || payload[1] !== 101) {
      return this.rejectBoard(`Unsafe or unsupported forwarded command ${command}`);
    }
    const packet = payload[2];
    if (packet !== undefined && [12, 20, 22, 36, 202].includes(packet) && controller.refloatControls) {
      const phase =
        controller.refloatControls.state.flywheelEnabled || controller.telemetry?.phase() === 'riding'
          ? 'riding'
          : 'idle';
      const control = controller.refloatControls.apply(payload, {
        ...(canId === undefined ? {} : { selectedCanId: canId, requestCanId: canId }),
        rideState: phase,
      });
      if (!control.accepted) return this.rejectBoard(`Rejected Refloat control ${String(control.reason)}`);
      return {
        response: control.response,
        request: { ...base, kind: 'refloat-control' },
      };
    }
    if (packet === 35 && controller.refloatControls) {
      const response = controller.refloatControls.alertsResponse(payload);
      if (!response) return this.rejectBoard('Unsupported Refloat alerts request');
      return { response, request: { ...base, kind: 'telemetry', telemetryCommand: 'alerts' } };
    }
    if (controller.refloat && controller.telemetry) {
      const isTelemetry = packet === 1 || packet === 10 || packet === 31 || packet === 33;
      const phase = controller.telemetry.phase();
      const sample = isTelemetry
        ? (controller.refloatControls?.applyToSample(controller.telemetry.next()) ?? controller.telemetry.next())
        : idleTelemetry;
      const response = controller.refloat.respond(payload, sample);
      if (!response) return this.rejectBoard(`Unsupported Refloat request ${String(packet)}`);
      const kind =
        response.kind === 'info' ? 'refloat-info' : response.kind === 'layout' ? 'refloat-layout' : 'telemetry';
      return {
        response:
          isTelemetry && controller.refloatControls
            ? controller.refloatControls.decorateTelemetry(response.payload)
            : response.payload,
        request: {
          ...base,
          kind,
          ...(kind === 'telemetry'
            ? {
                telemetryPhase: phase,
                telemetryCommand: packet === 1 ? 'legacy-realtime' : packet === 10 ? 'compact' : 'realtime',
              }
            : {}),
        },
      };
    }
    switch (payload[2]) {
      case 0:
        return {
          response: controller.refloatInfoPayload,
          request: { ...base, kind: 'refloat-info' },
        };
      case 32:
        return {
          response: controller.telemetryLayoutPayload,
          request: { ...base, kind: 'refloat-layout' },
        };
      case 31: {
        const phase = telemetryPhase(controller, this.telemetryIndex++);
        return {
          response: phase.payload,
          request: {
            ...base,
            kind: 'telemetry',
            telemetryPhase: phase.name,
            telemetryCommand: 'realtime',
          },
        };
      }
      case 10: {
        const phase = telemetryPhase(controller, this.telemetryIndex++);
        return {
          response: encodeLegacyCompact(idleTelemetry, payload[3] ?? 4),
          request: { ...base, kind: 'telemetry', telemetryPhase: phase.name, telemetryCommand: 'compact' },
        };
      }
      default:
        throw new Error(`Unsupported Refloat request ${String(payload[2])}`);
    }
  }

  private bmsResponse(
    bms: BmsConfiguration,
    payload: Uint8Array,
    at: number,
    canId?: number
  ): SimulatedTransaction | undefined {
    const command = payload[0];
    if (bms.failureMode === 'timeout') return undefined;
    if (command === 0) {
      return { response: bms.firmware.payload, request: { at, command, canId, kind: 'bms-firmware' } };
    }
    if (command !== 96 || payload.length !== 1) {
      return this.rejectBoard(`Unsafe or unsupported BMS command ${String(command)}`);
    }
    let response = encodeBmsValuesPayload(bms.valuesSource?.next() ?? bms.values, bms.tail);
    response = malformedBmsResponse(response, bms.failureMode ?? 'none');
    return { response, request: { at, command, canId, kind: 'bms-values' } };
  }
}

function malformedBmsResponse(payload: Uint8Array, mode: BmsConfiguration['failureMode']): Uint8Array {
  const result = payload.slice();
  const cellCount = result[25] ?? 0;
  const temperatureCountOffset = 26 + cellCount * 3;
  const temperatureCount = result[temperatureCountOffset] ?? 0;
  const reportedIdOffset = temperatureCountOffset + 1 + temperatureCount * 2 + 12;
  const lifetimeOffset = reportedIdOffset + 1;
  const fixedEnd = reportedIdOffset + 19;
  switch (mode) {
    case 'excessive-cells':
      result[25] = 51;
      return result;
    case 'excessive-temperatures':
      result[temperatureCountOffset] = 51;
      return result;
    case 'invalid-balancing':
      if (cellCount > 0) result[26 + cellCount * 2] = 2;
      return result;
    case 'truncate-fixed':
      return result.slice(0, 24);
    case 'truncate-cells':
      return result.slice(0, Math.max(26, 26 + cellCount * 2 - 1));
    case 'truncate-balancing':
      return result.slice(0, Math.max(26, temperatureCountOffset - 1));
    case 'truncate-temperatures':
      return result.slice(0, Math.max(temperatureCountOffset + 1, reportedIdOffset - 1));
    case 'truncate-tail':
      return result.slice(0, Math.max(0, fixedEnd - 1));
    case 'non-finite-auto-float':
      result.set([0x7f, 0x80, 0, 0], lifetimeOffset);
      return result;
    case 'reported-id-mismatch':
      result[reportedIdOffset] = ((result[reportedIdOffset] ?? 0) + 1) % 254;
      return result;
    case 'unterminated-status':
      return result[result.length - 1] === 0 ? result.slice(0, -1) : result;
    case 'oversized-status':
      return Uint8Array.from([...result.slice(0, fixedEnd), 1, ...new Uint8Array(41).fill(0x41), 0]);
    case 'invalid-status-utf8':
      return Uint8Array.from([...result.slice(0, fixedEnd), 1, 0xff, 0]);
    case 'unexpected-remainder':
      return Uint8Array.from([...result, 0]);
    default:
      return result;
  }
}

function requestAttempt(payload: Uint8Array): FixtureRequestAttempt {
  const forwarded = payload[0] === 34 && payload.length >= 3;
  const command = forwarded ? payload[2]! : (payload[0] ?? 255);
  const canId = forwarded ? payload[1] : undefined;
  const packetOffset = forwarded ? 4 : 2;
  const packet = command === 36 && payload[forwarded ? 3 : 1] === 101 ? payload[packetOffset] : undefined;
  const telemetryCommand =
    packet === 1
      ? 'legacy-realtime'
      : packet === 10
        ? 'compact'
        : packet === 31 || packet === 33
          ? 'realtime'
          : undefined;
  return {
    command,
    ...(canId === undefined ? {} : { canId }),
    ...(telemetryCommand === undefined ? {} : { telemetryCommand }),
  };
}

function telemetryPhase(controller: ControllerConfiguration, index: number) {
  let remaining = index;
  for (const phase of controller.telemetryPhases) {
    if (remaining < phase.repetitions) return phase;
    remaining -= phase.repetitions;
  }
  return (
    controller.telemetryPhases.at(-1) ??
    (() => {
      throw new Error('Controller has no telemetry phases');
    })()
  );
}

function matchesControllerCanIdRequest(payload: Uint8Array): boolean {
  return payload.length === 5 && payload.every((byte, index) => byte === [50, 0, 2, 0, 0][index]);
}
