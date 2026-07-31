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

export type PhysicalScenarioName =
  | 'identity'
  | 'direct'
  | 'direct-legacy'
  | 'explicit-can'
  | 'generic-gateway'
  | 'ambiguous'
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
}

export const fixtureScenarios: Record<PhysicalScenarioName, FixtureScenario> = {
  identity: {
    name: 'identity',
    board: expressThor,
    fragmentBytes: 244,
    minimumTelemetryResponses: 0,
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
  ride: {
    name: 'ride',
    board: expressThor,
    fragmentBytes: 20,
    minimumTelemetryResponses: 18,
    expectedTelemetryIntervalMs: 100,
    // Physical BLE startup can include one idle-to-riding transition cycle;
    // the scenario still requires the complete riding request count.
    cadenceToleranceMs: 400,
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
    'direct',
    'direct-legacy',
    'explicit-can',
    'generic-gateway',
    'ambiguous',
    'ride',
    'unexpected-disconnect',
  ],
  android: [
    'identity',
    'direct',
    'direct-legacy',
    'explicit-can',
    'generic-gateway',
    'ambiguous',
    'ride',
    'unexpected-disconnect',
    'background',
  ],
  ios: ['identity', 'direct', 'ride', 'unexpected-disconnect', 'background'],
} as const satisfies Record<string, readonly PhysicalScenarioName[]>;

export type SoftwareConnectionScenarioName =
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
  lifecycle: 'connect' | 'selection' | 'retry' | 'reconnect' | 'frontend-recovery';
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
