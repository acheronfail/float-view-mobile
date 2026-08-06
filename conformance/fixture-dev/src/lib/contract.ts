import type { FirmwareVersion } from '../../../fixture/firmware.ts';
import type { RefloatVersion } from '../../../fixture/refloat.ts';
import type { TelemetrySample, TelemetryScene } from '../../../fixture/telemetry.ts';
import type { FixtureTopology } from '../../../fixture/topology.ts';
import type { BmsFailureMode } from '../../../fixture/bms.ts';

export const FIXTURE_DEV_ORIGIN = 'http://127.0.0.1:11004';
export const FIXTURE_DEV_PORT = 11004;

export interface FixtureConfiguration {
  topology: FixtureTopology;
  canId: number;
  firmware: FirmwareVersion;
  refloat: RefloatVersion;
  bmsTopology: BmsTopology;
  bmsCanId: number;
  bmsVariant: 'legacy' | 'extended';
  bmsFailureMode: BmsFailureMode;
  bmsCellCount: number;
  bmsTemperatureCount: number;
  bmsStateOfCharge: number;
  bmsInputCurrentA: number;
}

export type BmsTopology = 'none' | 'can' | 'direct';

export interface ScenarioBmsNode {
  alias: string;
  route: 'direct' | 'can';
  canId: number;
  pingVisible: boolean;
  variant: 'legacy' | 'extended';
  status: string;
  failureMode: BmsFailureMode;
  seed: number;
  cellCount: number;
  temperatureCount: number;
  stateOfCharge: number;
  inputCurrentA: number;
}

export interface ScenarioController {
  alias: string;
  canId: number;
  firmware: FirmwareVersion;
  refloat: RefloatVersion;
  pingVisible: boolean;
}

export interface FixtureScenarioDocument {
  schemaVersion: 1;
  namespace: string;
  advertisedName: string;
  topology: FixtureTopology;
  fragmentBytes: number;
  controllers: ScenarioController[];
  bmsNodes: ScenarioBmsNode[];
  discoveredCanIds: number[];
  scene: TelemetryScene;
  overrides: Partial<TelemetrySample>;
}

export type ScenarioTemplate = 'direct' | 'singleExpress' | 'reportedMultiController' | 'directVbms';

export interface LastFixtureRequest {
  kind: string;
  command: number;
  canId?: number;
  telemetryCommand?: string;
  responseBytes: number;
  fragments: number;
}

export interface LastFixtureAttempt {
  command: number;
  canId?: number;
  telemetryCommand?: string;
  stage: 'simulate' | 'fragment' | 'notify';
  outcome: 'pending' | 'responded' | 'dropped' | 'failed';
  error?: string;
}

export interface SerialPortOption {
  path: string;
  label: string;
}

export interface FixtureCounters {
  writes: number;
  responses: number;
  telemetry: number;
  bms: number;
  dropped: number;
}

export interface FixtureSnapshot {
  revision: number;
  serial: {
    status: 'detached' | 'attaching' | 'attached';
    path?: string;
    error?: string;
  };
  configuration: FixtureConfiguration;
  power: 'off' | 'on';
  ble: 'unavailable' | 'idle' | 'advertising' | 'connected';
  ride: 'idle' | 'riding';
  telemetry?: TelemetrySample;
  scenario: FixtureScenarioDocument;
  frozen: boolean;
  pinnedFields: Partial<TelemetrySample>;
  availableFields: readonly (keyof TelemetrySample)[];
  lastAttempt?: LastFixtureAttempt;
  lastRequest?: LastFixtureRequest;
  counters: FixtureCounters;
  diagnostics: readonly string[];
}

export type FixtureAction =
  | { kind: 'attach'; path: string }
  | { kind: 'detach' }
  | { kind: 'configure'; configuration: FixtureConfiguration }
  | { kind: 'power-on' }
  | { kind: 'power-off' }
  | { kind: 'start-riding' }
  | { kind: 'stop-riding' }
  | { kind: 'apply-template'; template: ScenarioTemplate }
  | { kind: 'import-scenario'; document: unknown }
  | { kind: 'set-scene'; scene: TelemetryScene }
  | { kind: 'freeze' }
  | { kind: 'resume' }
  | { kind: 'pin-field'; field: keyof TelemetrySample; value: TelemetrySample[keyof TelemetrySample] }
  | { kind: 'unpin-field'; field: keyof TelemetrySample }
  | { kind: 'reset-overrides' }
  | { kind: 'disconnect-readvertise' }
  | { kind: 'reconnect-unchanged' }
  | { kind: 'move-controller'; alias: string; canId: number }
  | { kind: 'replace-controller'; alias: string }
  | { kind: 'set-ping-visible'; alias: string; visible: boolean }
  | { kind: 'set-fragment-bytes'; fragmentBytes: number };

export const firmwareOptions: readonly FirmwareVersion[] = ['6.02', '6.05', '6.06', '7.00'];
export const refloatOptions: readonly RefloatVersion[] = [
  '1.0.0',
  '1.0.1',
  '1.1.0',
  '1.1.1',
  '1.1.2',
  '1.2.0',
  '1.2.1',
  '1.2.2',
  '1.2.3',
];
export const topologyOptions: readonly FixtureTopology[] = ['direct', 'express'];
export const bmsTopologyOptions: readonly BmsTopology[] = ['none', 'can', 'direct'];
export const scenarioTemplateOptions: readonly ScenarioTemplate[] = [
  'direct',
  'singleExpress',
  'reportedMultiController',
  'directVbms',
];
