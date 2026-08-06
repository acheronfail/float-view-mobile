import { createHash } from 'node:crypto';
import type { RefloatVersion } from '../../../../fixture/refloat.ts';
import {
  idleTelemetry,
  InteractiveTelemetrySource,
  telemetryScenes,
  type TelemetrySample,
} from '../../../../fixture/telemetry.ts';
import { validateScenarioDocument } from '../../../../fixture/telemetry-contract.ts';
import { bmsFailureModes } from '../../../../fixture/bms.ts';
import { firmwareOptions, refloatOptions, type FixtureScenarioDocument, type ScenarioTemplate } from '../contract.ts';

const TOP_LEVEL_KEYS = new Set([
  'schemaVersion',
  'namespace',
  'advertisedName',
  'topology',
  'fragmentBytes',
  'controllers',
  'bmsNodes',
  'discoveredCanIds',
  'scene',
  'overrides',
]);
const CONTROLLER_KEYS = new Set(['alias', 'canId', 'firmware', 'refloat', 'pingVisible']);
const BMS_KEYS = new Set([
  'alias',
  'route',
  'canId',
  'pingVisible',
  'variant',
  'status',
  'failureMode',
  'seed',
  'cellCount',
  'temperatureCount',
  'stateOfCharge',
  'inputCurrentA',
]);
const TELEMETRY_FIELDS = new Set(Object.keys(idleTelemetry));
const utf8 = new TextEncoder();

export function scenarioTemplate(template: ScenarioTemplate): FixtureScenarioDocument {
  const common = {
    schemaVersion: 1 as const,
    namespace: 'float-view-workbench',
    advertisedName: 'FloatView Test VESC',
    fragmentBytes: 20,
    scene: 'idle' as const,
    overrides: {},
  };
  switch (template) {
    case 'direct':
      return {
        ...common,
        topology: 'direct',
        controllers: [{ alias: 'primary', canId: 0, firmware: '6.06', refloat: '1.2.3', pingVisible: false }],
        bmsNodes: [],
        discoveredCanIds: [],
      };
    case 'singleExpress':
      return {
        ...common,
        topology: 'express',
        controllers: [{ alias: 'primary', canId: 1, firmware: '6.06', refloat: '1.2.3', pingVisible: true }],
        bmsNodes: [],
        discoveredCanIds: [],
      };
    case 'reportedMultiController':
      return {
        ...common,
        namespace: 'reported-multi-controller',
        topology: 'express',
        controllers: [
          { alias: 'front', canId: 7, firmware: '6.05', refloat: '1.1.0', pingVisible: true },
          { alias: 'rear', canId: 42, firmware: '6.06', refloat: '1.2.3', pingVisible: false },
        ],
        bmsNodes: [],
        discoveredCanIds: [12],
      };
    case 'directVbms':
      return {
        ...common,
        namespace: 'direct-vbms-gateway',
        topology: 'express',
        controllers: [{ alias: 'primary', canId: 1, firmware: '6.06', refloat: '1.2.3', pingVisible: true }],
        bmsNodes: [defaultBmsNode('direct', 0)],
        discoveredCanIds: [],
      };
  }
}

export function parseScenarioDocument(value: unknown): FixtureScenarioDocument {
  const encoded = JSON.stringify(value);
  validateScenarioDocument(encoded);
  if (!record(value) || !onlyKeys(value, TOP_LEVEL_KEYS)) throw new Error('Invalid scenario document');
  if (value.schemaVersion !== 1) throw new Error('Unsupported scenario schema version');
  if (!boundedText(value.namespace, 1, 64) || !boundedText(value.advertisedName, 1, 64)) {
    throw new Error('Scenario namespace or advertised name exceeds its UTF-8 bound');
  }
  if (value.topology !== 'direct' && value.topology !== 'express') throw new Error('Invalid topology');
  if (!integer(value.fragmentBytes, 1, 244)) throw new Error('Invalid notification fragment bound');
  if (!Array.isArray(value.controllers) || value.controllers.length < 1 || value.controllers.length > 16) {
    throw new Error('Scenario requires 1 through 16 controller candidates');
  }
  if (!Array.isArray(value.discoveredCanIds) || value.discoveredCanIds.length > 64) {
    throw new Error('Scenario discovered CAN IDs exceed 64 entries');
  }
  if (!Array.isArray(value.bmsNodes) || value.bmsNodes.length > 16) {
    throw new Error('Scenario VBMS nodes exceed 16 entries');
  }
  if (typeof value.scene !== 'string' || !telemetryScenes.includes(value.scene as never)) {
    throw new Error('Invalid telemetry scene');
  }
  if (!record(value.overrides) || !onlyKeys(value.overrides, TELEMETRY_FIELDS)) {
    throw new Error('Invalid telemetry overrides');
  }

  const aliases = new Set<string>();
  const controllerIds = new Set<number>();
  const controllers = value.controllers.map((candidate) => {
    if (!record(candidate) || !onlyKeys(candidate, CONTROLLER_KEYS)) {
      throw new Error('Invalid controller candidate');
    }
    if (!boundedText(candidate.alias, 1, 64) || aliases.has(candidate.alias)) {
      throw new Error('Controller aliases must be unique and at most 64 UTF-8 bytes');
    }
    if (!integer(candidate.canId, 0, 253) || controllerIds.has(candidate.canId)) {
      throw new Error('Controller CAN IDs must be unique integers from 0 through 253');
    }
    if (
      typeof candidate.firmware !== 'string' ||
      !firmwareOptions.includes(candidate.firmware as never) ||
      typeof candidate.refloat !== 'string' ||
      !refloatOptions.includes(candidate.refloat as never) ||
      typeof candidate.pingVisible !== 'boolean'
    ) {
      throw new Error('Invalid controller source identity');
    }
    aliases.add(candidate.alias);
    controllerIds.add(candidate.canId);
    return {
      alias: candidate.alias,
      canId: candidate.canId,
      firmware: candidate.firmware as FixtureScenarioDocument['controllers'][number]['firmware'],
      refloat: candidate.refloat as RefloatVersion,
      pingVisible: candidate.pingVisible,
    };
  });

  const discovered = value.discoveredCanIds.map((canId) => {
    if (!integer(canId, 0, 253)) throw new Error('Discovered CAN IDs must be integers from 0 through 253');
    return canId;
  });
  if (new Set(discovered).size !== discovered.length) throw new Error('Discovered CAN IDs contain duplicates');
  if (discovered.some((canId) => controllerIds.has(canId))) {
    throw new Error('Controller and discovered CAN IDs must be unique');
  }

  const bmsAliases = new Set<string>();
  const bmsCanIds = new Set<number>();
  let directBmsCount = 0;
  const bmsNodes = value.bmsNodes.map((node) => {
    if (!record(node) || !onlyKeys(node, BMS_KEYS)) throw new Error('Invalid VBMS node');
    if (!boundedText(node.alias, 1, 64) || aliases.has(node.alias) || bmsAliases.has(node.alias)) {
      throw new Error('VBMS aliases must be unique and at most 64 UTF-8 bytes');
    }
    if (node.route !== 'direct' && node.route !== 'can') throw new Error('Invalid VBMS route');
    if (!integer(node.canId, 0, 253)) throw new Error('VBMS CAN IDs must be integers from 0 through 253');
    if (node.route === 'direct') directBmsCount += 1;
    if (
      node.route === 'can' &&
      (controllerIds.has(node.canId) || bmsCanIds.has(node.canId) || discovered.includes(node.canId))
    ) {
      throw new Error('Controller, VBMS, and discovered CAN IDs must be unique');
    }
    if (typeof node.pingVisible !== 'boolean') throw new Error('Invalid VBMS ping visibility');
    if (node.variant !== 'legacy' && node.variant !== 'extended') throw new Error('Invalid VBMS response variant');
    if (!boundedText(node.status, 0, 40)) throw new Error('VBMS status exceeds 40 UTF-8 bytes');
    if (typeof node.failureMode !== 'string' || !bmsFailureModes.includes(node.failureMode as never)) {
      throw new Error('Invalid VBMS failure mode');
    }
    if (!integer(node.seed, 0, 0x7fffffff)) throw new Error('Invalid VBMS seed');
    if (!integer(node.cellCount, 1, 50) || !integer(node.temperatureCount, 0, 50)) {
      throw new Error('VBMS array counts exceed source bounds');
    }
    if (!finiteRange(node.stateOfCharge, 0, 1) || !finiteRange(node.inputCurrentA, -200, 200)) {
      throw new Error('Invalid VBMS values');
    }
    bmsAliases.add(node.alias);
    if (node.route === 'can') bmsCanIds.add(node.canId);
    return {
      alias: node.alias,
      route: node.route,
      canId: node.canId,
      pingVisible: node.pingVisible,
      variant: node.variant,
      status: node.status,
      failureMode: node.failureMode,
      seed: node.seed,
      cellCount: node.cellCount,
      temperatureCount: node.temperatureCount,
      stateOfCharge: node.stateOfCharge,
      inputCurrentA: node.inputCurrentA,
    } as FixtureScenarioDocument['bmsNodes'][number];
  });
  if (directBmsCount > 1) throw new Error('Only one direct VBMS endpoint is allowed');
  if (directBmsCount === 1 && value.topology !== 'express') {
    throw new Error('Direct VBMS topology requires a CAN-routed controller');
  }
  if (value.topology === 'direct' && (controllers.length !== 1 || discovered.length !== 0)) {
    throw new Error('Direct topology requires exactly one controller and no discovered CAN IDs');
  }

  const source = new InteractiveTelemetrySource();
  source.setScene(value.scene as FixtureScenarioDocument['scene']);
  const overrides: Partial<TelemetrySample> = {};
  for (const [field, fieldValue] of Object.entries(value.overrides)) {
    source.pin(field as keyof TelemetrySample, fieldValue as never);
    (overrides as Record<string, unknown>)[field] = fieldValue;
  }
  return {
    schemaVersion: 1,
    namespace: value.namespace,
    advertisedName: value.advertisedName,
    topology: value.topology,
    fragmentBytes: value.fragmentBytes,
    controllers,
    bmsNodes,
    discoveredCanIds: discovered,
    scene: value.scene as FixtureScenarioDocument['scene'],
    overrides,
  };
}

export function privateControllerIdentity(document: FixtureScenarioDocument, alias: string): Uint8Array {
  const digest = createHash('sha256').update(document.namespace).update('\0').update(alias).digest();
  return Uint8Array.from(digest.subarray(0, 12));
}

export const privateBmsIdentity = privateControllerIdentity;

export function defaultBmsNode(
  route: 'direct' | 'can' = 'can',
  canId = 42
): FixtureScenarioDocument['bmsNodes'][number] {
  return {
    alias: route === 'direct' ? 'local-vbms' : `vbms-${canId}`,
    route,
    canId,
    pingVisible: route === 'can',
    variant: 'extended',
    status: 'OK',
    failureMode: 'none',
    seed: 42,
    cellCount: 18,
    temperatureCount: 4,
    stateOfCharge: 0.8,
    inputCurrentA: -4,
  };
}

export function availableTelemetryFields(version: RefloatVersion): readonly (keyof TelemetrySample)[] {
  const fields = Object.keys(idleTelemetry) as (keyof TelemetrySample)[];
  return version === '1.1.0' ? fields.filter((field) => field !== 'firmwareFault') : fields;
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function onlyKeys(value: Record<string, unknown>, keys: ReadonlySet<string>) {
  return Object.keys(value).every((key) => keys.has(key));
}

function boundedText(value: unknown, minimum: number, maximum: number): value is string {
  return typeof value === 'string' && utf8.encode(value).length >= minimum && utf8.encode(value).length <= maximum;
}

function integer(value: unknown, minimum: number, maximum: number): value is number {
  return Number.isInteger(value) && Number(value) >= minimum && Number(value) <= maximum;
}

function finiteRange(value: unknown, minimum: number, maximum: number): value is number {
  return typeof value === 'number' && Number.isFinite(value) && value >= minimum && value <= maximum;
}
