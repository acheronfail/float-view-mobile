import {
  ambiguousThor,
  directLegacyThor,
  directThor,
  expressExplicitThor,
  expressTransport,
  expressThor,
  genericGatewayThor,
  type BoardConfiguration,
} from './boards.ts';
import { encodeFirmwarePayload, thor300 } from './firmware.ts';
import type { ConnectionIssueCode } from '../../frontend/src/lib/backend/contract.ts';
import { ControllerShutdownModel } from './controller-shutdown.ts';
import { RefloatControlModel } from './refloat-controls.ts';
import { RefloatConfigurationModel } from './refloat-configuration.ts';
import { refloatCodecs } from './refloat.ts';
import { idleTelemetry, InteractiveTelemetrySource, type TelemetrySource } from './telemetry.ts';
import {
  encodeBmsFirmwarePayload,
  encodeVbms32FirmwarePayload,
  typicalBmsValues,
  type BmsConfiguration,
} from './bms.ts';

export type PhysicalScenarioName =
  | 'auto-connect'
  | 'identity'
  | 'guest'
  | 'direct'
  | 'direct-legacy'
  | 'fault-direct'
  | 'fault-can'
  | 'control-direct'
  | 'control-can'
  | 'shutdown-direct'
  | 'shutdown-can'
  | 'shutdown-no-op'
  | 'explicit-can'
  | 'generic-gateway'
  | 'ambiguous'
  | 'bms-direct-controller'
  | 'bms-express'
  | 'bms-standalone'
  | 'ride'
  | 'unexpected-disconnect'
  | 'background';

export interface FixtureScenario {
  name: PhysicalScenarioName;
  board: BoardConfiguration;
  fragmentBytes: number;
  minimumTelemetryResponses: number;
  expectedTelemetryIntervalMs?: number;
  cadenceToleranceMs?: number;
  disconnectAfterTelemetry?: number;
  requiresMobileLifecycle: boolean;
  preferredCanId?: number | null;
  selectCanId?: number;
  expectedBmsConnection?: 'direct' | { canId: number };
}

const controlDirectControllers = directThor.controllers.map((controller) => ({
  ...controller,
  firmware: {
    ...controller.firmware,
    payload: encodeFirmwarePayload(thor300, 6, 6, Uint8Array.of(0, 0, 0, 0), 0, 1),
  },
  refloatVersion: '1.3.0',
  refloatInfoPayload: refloatCodecs['1.3.0'].infoPayload,
  telemetryLayoutPayload: new Uint8Array(),
  refloat: refloatCodecs['1.3.0'],
  refloatControls: new RefloatControlModel('1.3.0'),
  refloatConfiguration: new RefloatConfigurationModel('1.3.0'),
  telemetry: idleControlTelemetry(),
}));

const controlDirectBoard: BoardConfiguration = {
  ...directThor,
  name: 'Direct / Refloat product controls',
  transport: controlDirectControllers[0]!.firmware,
  controllers: controlDirectControllers,
};

const controlCanBoard: BoardConfiguration = {
  ...expressExplicitThor,
  name: 'Express CAN / Refloat product controls',
  controllers: expressExplicitThor.controllers.map((controller) => ({
    ...controller,
    firmware: {
      ...controller.firmware,
      payload: encodeFirmwarePayload(thor300, 6, 6, Uint8Array.of(0, 0, 0, 0), 0, 1),
    },
    refloatVersion: '1.3.0',
    refloatInfoPayload: refloatCodecs['1.3.0'].infoPayload,
    telemetryLayoutPayload: new Uint8Array(),
    refloat: refloatCodecs['1.3.0'],
    refloatControls: new RefloatControlModel('1.3.0'),
    refloatConfiguration: new RefloatConfigurationModel('1.3.0'),
    telemetry: idleControlTelemetry(),
  })),
};

function idleControlTelemetry(): TelemetrySource {
  return {
    phase: () => 'idle',
    next: () => idleTelemetry,
  };
}

const shutdownDirectBoard: BoardConfiguration = {
  ...controlDirectBoard,
  name: 'Direct / controller shutdown',
  controllers: controlDirectBoard.controllers.map((controller) => ({
    ...controller,
    shutdown: new ControllerShutdownModel('disconnect'),
  })),
};

const shutdownCanBoard: BoardConfiguration = {
  ...controlCanBoard,
  name: 'Express CAN / controller shutdown',
  controllers: controlCanBoard.controllers.map((controller) => ({
    ...controller,
    shutdown: new ControllerShutdownModel('disconnect'),
  })),
};

const shutdownNoOpBoard: BoardConfiguration = {
  ...controlDirectBoard,
  name: 'Direct / unsupported hardware shutdown no-op',
  controllers: controlDirectBoard.controllers.map((controller) => ({
    ...controller,
    shutdown: new ControllerShutdownModel('no-op'),
  })),
};

const physicalCanBms: BmsConfiguration = {
  canId: 42,
  firmware: {
    major: 6,
    minor: 6,
    hardwareName: 'VBMS32',
    hardwareType: 2,
    payload: encodeVbms32FirmwarePayload(Uint8Array.of(0, 0x11, 0x22, 0x33, 0x44, 0x55)),
  },
  values: typicalBmsValues,
  tail: { kind: 'extended', dataVersion: 1, status: 'OK' },
};

const physicalDirectBms: BmsConfiguration = {
  firmware: {
    major: 6,
    minor: 0,
    hardwareName: 'VBMS',
    hardwareType: 1,
    payload: encodeBmsFirmwarePayload('VBMS', new Uint8Array(12).fill(0x22)),
  },
  values: typicalBmsValues,
  tail: { kind: 'legacy' },
};

export const fixtureScenarios: Record<PhysicalScenarioName, FixtureScenario> = {
  'auto-connect': {
    name: 'auto-connect',
    board: directThor,
    fragmentBytes: 20,
    minimumTelemetryResponses: 1,
    preferredCanId: null,
    requiresMobileLifecycle: true,
  },
  identity: {
    name: 'identity',
    board: expressThor,
    fragmentBytes: 20,
    minimumTelemetryResponses: 0,
    preferredCanId: null,
    requiresMobileLifecycle: false,
  },
  guest: {
    name: 'guest',
    board: directThor,
    fragmentBytes: 20,
    minimumTelemetryResponses: 5,
    preferredCanId: null,
    requiresMobileLifecycle: false,
  },
  direct: {
    name: 'direct',
    board: directThor,
    fragmentBytes: 20,
    minimumTelemetryResponses: 2,
    preferredCanId: null,
    requiresMobileLifecycle: false,
  },
  'direct-legacy': {
    name: 'direct-legacy',
    board: directLegacyThor,
    fragmentBytes: 20,
    minimumTelemetryResponses: 2,
    preferredCanId: null,
    requiresMobileLifecycle: false,
  },
  'fault-direct': {
    name: 'fault-direct',
    board: controlDirectBoard,
    fragmentBytes: 20,
    minimumTelemetryResponses: 6,
    preferredCanId: null,
    requiresMobileLifecycle: false,
  },
  'fault-can': {
    name: 'fault-can',
    board: { ...controlCanBoard, canPingIds: [42] },
    fragmentBytes: 20,
    minimumTelemetryResponses: 6,
    preferredCanId: null,
    requiresMobileLifecycle: false,
  },
  'control-direct': {
    name: 'control-direct',
    board: controlDirectBoard,
    fragmentBytes: 20,
    minimumTelemetryResponses: 6,
    preferredCanId: null,
    requiresMobileLifecycle: false,
  },
  'control-can': {
    name: 'control-can',
    board: controlCanBoard,
    fragmentBytes: 20,
    minimumTelemetryResponses: 6,
    preferredCanId: 42,
    requiresMobileLifecycle: false,
  },
  'shutdown-direct': {
    name: 'shutdown-direct',
    board: shutdownDirectBoard,
    fragmentBytes: 20,
    minimumTelemetryResponses: 1,
    preferredCanId: null,
    requiresMobileLifecycle: false,
  },
  'shutdown-can': {
    name: 'shutdown-can',
    board: shutdownCanBoard,
    fragmentBytes: 20,
    minimumTelemetryResponses: 1,
    preferredCanId: 42,
    requiresMobileLifecycle: false,
  },
  'shutdown-no-op': {
    name: 'shutdown-no-op',
    board: shutdownNoOpBoard,
    fragmentBytes: 20,
    minimumTelemetryResponses: 1,
    preferredCanId: null,
    requiresMobileLifecycle: false,
  },
  'explicit-can': {
    name: 'explicit-can',
    board: expressExplicitThor,
    fragmentBytes: 20,
    minimumTelemetryResponses: 2,
    preferredCanId: 42,
    requiresMobileLifecycle: false,
  },
  'generic-gateway': {
    name: 'generic-gateway',
    board: genericGatewayThor,
    fragmentBytes: 20,
    minimumTelemetryResponses: 2,
    preferredCanId: null,
    requiresMobileLifecycle: false,
  },
  ambiguous: {
    name: 'ambiguous',
    board: ambiguousThor,
    fragmentBytes: 20,
    minimumTelemetryResponses: 2,
    preferredCanId: null,
    selectCanId: 42,
    requiresMobileLifecycle: false,
  },
  'bms-direct-controller': {
    name: 'bms-direct-controller',
    board: { ...directThor, name: 'Direct controller / CAN VBMS', bmsNodes: [physicalCanBms] },
    fragmentBytes: 20,
    minimumTelemetryResponses: 5,
    preferredCanId: null,
    expectedBmsConnection: { canId: 42 },
    requiresMobileLifecycle: false,
  },
  'bms-express': {
    name: 'bms-express',
    board: { ...expressThor, name: 'Express / controller / CAN VBMS', bmsNodes: [physicalCanBms] },
    fragmentBytes: 20,
    minimumTelemetryResponses: 5,
    preferredCanId: null,
    expectedBmsConnection: { canId: 42 },
    requiresMobileLifecycle: false,
  },
  'bms-standalone': {
    name: 'bms-standalone',
    board: {
      ...directThor,
      name: 'Standalone direct VBMS',
      transport: physicalDirectBms.firmware,
      controllers: [],
      localBms: physicalDirectBms,
      canPingIds: [],
    },
    fragmentBytes: 20,
    minimumTelemetryResponses: 0,
    preferredCanId: null,
    expectedBmsConnection: 'direct',
    requiresMobileLifecycle: false,
  },
  ride: {
    name: 'ride',
    board: expressThor,
    fragmentBytes: 20,
    minimumTelemetryResponses: 18,
    expectedTelemetryIntervalMs: 100,
    // Physical BLE startup can include one idle-to-riding transition cycle;
    // the scenario still requires the complete riding request count.
    cadenceToleranceMs: 600,
    preferredCanId: null,
    requiresMobileLifecycle: false,
  },
  'unexpected-disconnect': {
    name: 'unexpected-disconnect',
    board: expressThor,
    fragmentBytes: 20,
    minimumTelemetryResponses: 6,
    disconnectAfterTelemetry: 4,
    preferredCanId: null,
    requiresMobileLifecycle: false,
  },
  background: {
    name: 'background',
    board: expressThor,
    fragmentBytes: 20,
    minimumTelemetryResponses: 8,
    preferredCanId: null,
    requiresMobileLifecycle: true,
  },
};

export const physicalScenarioMatrix = {
  macos: [
    'identity',
    'guest',
    'direct',
    'direct-legacy',
    'fault-direct',
    'fault-can',
    'control-direct',
    'control-can',
    'shutdown-direct',
    'shutdown-can',
    'shutdown-no-op',
    'explicit-can',
    'generic-gateway',
    'ambiguous',
    'bms-direct-controller',
    'bms-express',
    'bms-standalone',
    'ride',
    'unexpected-disconnect',
  ],
  android: [
    'identity',
    'guest',
    'direct',
    'direct-legacy',
    'fault-direct',
    'fault-can',
    'control-direct',
    'control-can',
    'shutdown-direct',
    'shutdown-can',
    'shutdown-no-op',
    'explicit-can',
    'generic-gateway',
    'ambiguous',
    'bms-direct-controller',
    'bms-express',
    'bms-standalone',
    'ride',
    'unexpected-disconnect',
    'background',
    'auto-connect',
  ],
  ios: [
    'identity',
    'guest',
    'direct',
    'direct-legacy',
    'fault-direct',
    'fault-can',
    'control-direct',
    'control-can',
    'shutdown-direct',
    'shutdown-can',
    'shutdown-no-op',
    'explicit-can',
    'generic-gateway',
    'ambiguous',
    'bms-direct-controller',
    'bms-express',
    'bms-standalone',
    'ride',
    'unexpected-disconnect',
    'background',
    'auto-connect',
  ],
} as const satisfies Record<string, readonly PhysicalScenarioName[]>;

export const canonicalCanBms: BmsConfiguration = physicalCanBms;

export const canonicalDirectBms: BmsConfiguration = physicalDirectBms;

export type SoftwareBmsScenarioName =
  | 'direct-controller-plus-can-vbms'
  | 'express-plus-controller-and-vbms'
  | 'standalone-direct-vbms'
  | 'no-bms'
  | 'unique-automatic-selection'
  | 'configured-selection-without-ping'
  | 'wrong-preference'
  | 'multiple-ambiguity'
  | 'id-overflow'
  | 'candidate-overflow'
  | 'loss'
  | 'recovery'
  | 'reconnect'
  | 'disconnect';

export interface SoftwareBmsScenario {
  name: SoftwareBmsScenarioName;
  board: BoardConfiguration;
  preferredBmsCanId?: number | null;
  expectedStatus: 'not_found' | 'attached' | 'unavailable' | 'ambiguous';
  expectedRoute?: 'direct' | { canId: number };
  lifecycle: 'connect' | 'loss' | 'recovery' | 'reconnect' | 'disconnect';
}

const directControllerCanBms: BoardConfiguration = {
  ...directThor,
  name: 'Direct controller / CAN VBMS',
  bmsNodes: [canonicalCanBms],
  requestPolicy: 'interactive',
};

const expressControllerCanBms: BoardConfiguration = {
  ...expressThor,
  name: 'Express / controller / CAN VBMS',
  bmsNodes: [canonicalCanBms],
  requestPolicy: 'interactive',
};

const standaloneDirectBms: BoardConfiguration = {
  ...directThor,
  name: 'Standalone direct VBMS',
  transport: canonicalDirectBms.firmware,
  controllers: [],
  localBms: canonicalDirectBms,
  canPingIds: [],
  requestPolicy: 'interactive',
};

const secondCanBms: BmsConfiguration = {
  ...canonicalCanBms,
  canId: 43,
  values: { ...typicalBmsValues, reportedCanId: 43 },
  firmware: {
    ...canonicalCanBms.firmware,
    payload: encodeVbms32FirmwarePayload(new Uint8Array(6).fill(43)),
  },
};

const timeoutCanBms: BmsConfiguration = { ...canonicalCanBms, failureMode: 'timeout' };

export const softwareBmsScenarios: Record<SoftwareBmsScenarioName, SoftwareBmsScenario> = {
  'direct-controller-plus-can-vbms': {
    name: 'direct-controller-plus-can-vbms',
    board: directControllerCanBms,
    expectedStatus: 'attached',
    expectedRoute: { canId: 42 },
    lifecycle: 'connect',
  },
  'express-plus-controller-and-vbms': {
    name: 'express-plus-controller-and-vbms',
    board: expressControllerCanBms,
    expectedStatus: 'attached',
    expectedRoute: { canId: 42 },
    lifecycle: 'connect',
  },
  'standalone-direct-vbms': {
    name: 'standalone-direct-vbms',
    board: standaloneDirectBms,
    expectedStatus: 'attached',
    expectedRoute: 'direct',
    lifecycle: 'connect',
  },
  'no-bms': {
    name: 'no-bms',
    board: { ...expressThor, requestPolicy: 'interactive' },
    expectedStatus: 'not_found',
    lifecycle: 'connect',
  },
  'unique-automatic-selection': {
    name: 'unique-automatic-selection',
    board: expressControllerCanBms,
    preferredBmsCanId: null,
    expectedStatus: 'attached',
    expectedRoute: { canId: 42 },
    lifecycle: 'connect',
  },
  'configured-selection-without-ping': {
    name: 'configured-selection-without-ping',
    board: { ...expressControllerCanBms, canPingIds: [1] },
    preferredBmsCanId: 42,
    expectedStatus: 'attached',
    expectedRoute: { canId: 42 },
    lifecycle: 'connect',
  },
  'wrong-preference': {
    name: 'wrong-preference',
    board: expressControllerCanBms,
    preferredBmsCanId: 43,
    expectedStatus: 'not_found',
    lifecycle: 'connect',
  },
  'multiple-ambiguity': {
    name: 'multiple-ambiguity',
    board: { ...expressControllerCanBms, bmsNodes: [canonicalCanBms, secondCanBms] },
    expectedStatus: 'ambiguous',
    lifecycle: 'connect',
  },
  'id-overflow': {
    name: 'id-overflow',
    board: {
      ...expressControllerCanBms,
      canPingIds: Array.from({ length: 65 }, (_, index) => index),
    },
    expectedStatus: 'unavailable',
    lifecycle: 'connect',
  },
  'candidate-overflow': {
    name: 'candidate-overflow',
    board: {
      ...expressControllerCanBms,
      bmsNodes: Array.from({ length: 17 }, (_, index) => ({
        ...canonicalCanBms,
        canId: index + 20,
        values: { ...typicalBmsValues, reportedCanId: index + 20 },
      })),
    },
    expectedStatus: 'unavailable',
    lifecycle: 'connect',
  },
  loss: {
    name: 'loss',
    board: { ...expressControllerCanBms, bmsNodes: [timeoutCanBms] },
    expectedStatus: 'unavailable',
    expectedRoute: { canId: 42 },
    lifecycle: 'loss',
  },
  recovery: {
    name: 'recovery',
    board: expressControllerCanBms,
    expectedStatus: 'attached',
    expectedRoute: { canId: 42 },
    lifecycle: 'recovery',
  },
  reconnect: {
    name: 'reconnect',
    board: expressControllerCanBms,
    expectedStatus: 'attached',
    expectedRoute: { canId: 42 },
    lifecycle: 'reconnect',
  },
  disconnect: {
    name: 'disconnect',
    board: expressControllerCanBms,
    expectedStatus: 'unavailable',
    expectedRoute: { canId: 42 },
    lifecycle: 'disconnect',
  },
};

export type SoftwareConnectionScenarioName =
  | 'initial-firmware-retry'
  | 'automatic-can-discovery'
  | 'private-hint-match'
  | 'private-hint-fallback'
  | 'configured-can-without-ping'
  | 'wrong-configured-can'
  | 'generic-gateway-forwarding'
  | 'mixed-can-nodes'
  | 'zero-candidates'
  | 'can-id-overflow'
  | 'candidate-overflow'
  | 'supported-unsupported-ambiguity'
  | 'selection-validation'
  | 'selection-atomic-persistence'
  | 'failed-retry'
  | 'reconnect-continuity'
  | 'frontend-recovery';

export interface SoftwareConnectionScenario {
  name: SoftwareConnectionScenarioName;
  board: BoardConfiguration;
  preferredCanId?: number | null;
  privateHintCanId?: number;
  selectedCanId?: number;
  expectedRoute?: 'direct' | { canId: number };
  expectedIssue?: ConnectionIssueCode;
  expectsPing: boolean;
  lifecycle: 'connect' | 'initial-retry' | 'selection' | 'retry' | 'reconnect' | 'frontend-recovery';
}

const nonMotorNode = {
  ...expressThor.controllers[0]!,
  canId: 7,
  firmware: expressTransport,
};

const supportedController = (canId: number) => ({
  ...expressThor.controllers[0]!,
  canId,
  firmware: {
    ...expressThor.controllers[0]!.firmware,
    payload: encodeFirmwarePayload(
      { ...thor300, uuid: new Uint8Array(12).fill(canId + 1) },
      6,
      6,
      Uint8Array.of(0, 0, 0, 0)
    ),
  },
});

const mixedCanBoard: BoardConfiguration = {
  ...expressThor,
  name: 'Express / mixed CAN nodes',
  controllers: [nonMotorNode, supportedController(42)],
  canPingIds: [42, 7, 42, 254, 255],
  requestPolicy: 'interactive',
};

const zeroCandidateBoard: BoardConfiguration = {
  ...expressThor,
  name: 'Express / no motor controllers',
  controllers: [nonMotorNode],
  canPingIds: [7],
  requestPolicy: 'interactive',
};

const excessiveCanBoard: BoardConfiguration = {
  ...expressThor,
  name: 'Express / excessive CAN IDs',
  controllers: [],
  canPingIds: Array.from({ length: 65 }, (_, index) => index),
  requestPolicy: 'interactive',
};

const excessiveControllerBoard: BoardConfiguration = {
  ...expressThor,
  name: 'Express / excessive motor controllers',
  controllers: Array.from({ length: 17 }, (_, canId) => supportedController(canId)),
  requestPolicy: 'interactive',
};

const staleHintBoard: BoardConfiguration = {
  ...expressThor,
  name: 'Express / stale hint fallback',
  controllers: [nonMotorNode, supportedController(42)],
  canPingIds: [7, 42],
  requestPolicy: 'interactive',
};

export const softwareConnectionScenarios: Record<SoftwareConnectionScenarioName, SoftwareConnectionScenario> = {
  'initial-firmware-retry': {
    name: 'initial-firmware-retry',
    board: { ...directThor, dropFirstTransportFirmwareResponse: true },
    expectedRoute: 'direct',
    expectsPing: false,
    lifecycle: 'initial-retry',
  },
  'automatic-can-discovery': {
    name: 'automatic-can-discovery',
    board: expressThor,
    preferredCanId: null,
    expectedRoute: { canId: 1 },
    expectsPing: true,
    lifecycle: 'connect',
  },
  'private-hint-match': {
    name: 'private-hint-match',
    board: expressExplicitThor,
    privateHintCanId: 42,
    expectedRoute: { canId: 42 },
    expectsPing: false,
    lifecycle: 'connect',
  },
  'private-hint-fallback': {
    name: 'private-hint-fallback',
    board: staleHintBoard,
    privateHintCanId: 7,
    expectedRoute: { canId: 42 },
    expectsPing: true,
    lifecycle: 'connect',
  },
  'configured-can-without-ping': {
    name: 'configured-can-without-ping',
    board: expressExplicitThor,
    preferredCanId: 42,
    expectedRoute: { canId: 42 },
    expectsPing: false,
    lifecycle: 'connect',
  },
  'wrong-configured-can': {
    name: 'wrong-configured-can',
    board: { ...expressExplicitThor, requestPolicy: 'interactive' },
    preferredCanId: 7,
    expectedIssue: 'preferred_controller_unavailable',
    expectsPing: false,
    lifecycle: 'connect',
  },
  'generic-gateway-forwarding': {
    name: 'generic-gateway-forwarding',
    board: genericGatewayThor,
    expectedRoute: { canId: 1 },
    expectsPing: true,
    lifecycle: 'connect',
  },
  'mixed-can-nodes': {
    name: 'mixed-can-nodes',
    board: mixedCanBoard,
    expectedRoute: { canId: 42 },
    expectsPing: true,
    lifecycle: 'connect',
  },
  'zero-candidates': {
    name: 'zero-candidates',
    board: zeroCandidateBoard,
    expectedIssue: 'no_controller_found',
    expectsPing: true,
    lifecycle: 'connect',
  },
  'can-id-overflow': {
    name: 'can-id-overflow',
    board: excessiveCanBoard,
    expectedIssue: 'too_many_can_devices',
    expectsPing: true,
    lifecycle: 'connect',
  },
  'candidate-overflow': {
    name: 'candidate-overflow',
    board: excessiveControllerBoard,
    expectedIssue: 'too_many_controllers',
    expectsPing: true,
    lifecycle: 'connect',
  },
  'supported-unsupported-ambiguity': {
    name: 'supported-unsupported-ambiguity',
    board: ambiguousThor,
    selectedCanId: 42,
    expectedRoute: { canId: 42 },
    expectsPing: true,
    lifecycle: 'selection',
  },
  'selection-validation': {
    name: 'selection-validation',
    board: ambiguousThor,
    selectedCanId: 42,
    expectedRoute: { canId: 42 },
    expectsPing: true,
    lifecycle: 'selection',
  },
  'selection-atomic-persistence': {
    name: 'selection-atomic-persistence',
    board: ambiguousThor,
    selectedCanId: 42,
    expectedRoute: { canId: 42 },
    expectsPing: true,
    lifecycle: 'selection',
  },
  'failed-retry': {
    name: 'failed-retry',
    board: { ...expressExplicitThor, requestPolicy: 'interactive' },
    preferredCanId: 7,
    expectedIssue: 'preferred_controller_unavailable',
    expectsPing: false,
    lifecycle: 'retry',
  },
  'reconnect-continuity': {
    name: 'reconnect-continuity',
    board: expressThor,
    expectedRoute: { canId: 1 },
    expectsPing: true,
    lifecycle: 'reconnect',
  },
  'frontend-recovery': {
    name: 'frontend-recovery',
    board: ambiguousThor,
    selectedCanId: 42,
    expectedRoute: { canId: 42 },
    expectsPing: true,
    lifecycle: 'frontend-recovery',
  },
};

/** Select the audited package without replacing the older physical regression matrix. */
export function physicalScenario(name: PhysicalScenarioName, refloatVersion?: string): FixtureScenario {
  const scenario = fixtureScenarios[name];
  if (!refloatVersion) return scenario;
  if (refloatVersion !== '1.3.0') throw new Error('Physical Refloat override must be exactly 1.3.0');
  if (name === 'direct-legacy' || name === 'bms-standalone') return scenario;
  const codec = refloatCodecs['1.3.0'];
  return {
    ...scenario,
    board: {
      ...scenario.board,
      controllers: scenario.board.controllers.map((controller) => {
        const telemetry = new InteractiveTelemetrySource();
        if (!name.startsWith('fault-') && !name.startsWith('control-') && !name.startsWith('shutdown-'))
          telemetry.start();
        return {
          ...controller,
          refloatVersion: codec.version,
          refloatInfoPayload: codec.infoPayload,
          telemetryLayoutPayload: new Uint8Array(),
          refloat: codec,
          refloatControls: new RefloatControlModel('1.3.0'),
          refloatConfiguration: new RefloatConfigurationModel('1.3.0'),
          telemetry,
        };
      }),
    },
  };
}
