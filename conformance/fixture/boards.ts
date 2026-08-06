import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { FrameAssembler } from './frame.ts';
import {
  encodeFirmwarePayload,
  firmwareVariants,
  thor300,
  type ControllerIdentity,
  type FirmwareVariant,
  type FirmwareVersion,
} from './firmware.ts';
import { encodeRefloatTelemetry, refloatCodecs, type RefloatCodec, type RefloatVersion } from './refloat.ts';
import { RefloatControlModel } from './refloat-controls.ts';
import { idleTelemetry, type TelemetrySource } from './telemetry.ts';
import { directTopology, expressTopology, type FixtureTopology, type TopologyStrategy } from './topology.ts';
import type { BmsConfiguration } from './bms.ts';
import { ControllerShutdownModel, type ControllerShutdownBehavior } from './controller-shutdown.ts';

export interface FirmwareIdentity {
  major: number;
  minor: number;
  hardwareName: string;
  hardwareType: number;
  payload: Uint8Array;
}

export interface TelemetryPhase {
  name: 'idle' | 'riding' | 'footpad-warning' | 'post-ride-tail' | 'return-to-idle';
  repetitions: number;
  payload: Uint8Array;
}

export interface ControllerConfiguration {
  canId: number;
  firmware: FirmwareIdentity;
  refloatVersion: string;
  refloatInfoPayload: Uint8Array;
  telemetryLayoutPayload: Uint8Array;
  telemetryPhases: TelemetryPhase[];
  /** Interactive workbench controllers project one shared semantic source. */
  refloat?: RefloatCodec;
  refloatControls?: RefloatControlModel;
  telemetry?: TelemetrySource;
  shutdown?: ControllerShutdownModel;
}

export interface BoardConfiguration {
  name: string;
  advertisedName: string;
  transport: FirmwareIdentity;
  controllers: ControllerConfiguration[];
  /** Optional direct VBMS endpoint when the transport identity is hardware type 1. */
  localBms?: BmsConfiguration;
  /** Zero or more CAN-routed VBMS nodes, independently selected from controllers. */
  bmsNodes?: BmsConfiguration[];
  /** Raw COMM_PING_CAN response, including deliberate duplicates/invalid IDs. */
  canPingIds?: number[];
  /** Targets that deliberately omit a response to exercise bounded timeouts. */
  silentCanIds?: number[];
  /** Drops only the first local firmware response to exercise bounded startup recovery. */
  dropFirstTransportFirmwareResponse?: boolean;
  requestPolicy?: 'strict' | 'interactive';
}

export interface ComposedBoardConfiguration {
  kind: 'composed';
  name: string;
  advertisedName: string;
  controller: ControllerIdentity;
  firmware: FirmwareVariant;
  refloat: RefloatCodec;
  refloatControls: RefloatControlModel;
  topology: TopologyStrategy;
  telemetry: TelemetrySource;
  requestPolicy: 'strict' | 'interactive';
  expressTransport: FirmwareIdentity;
  shutdown: ControllerShutdownModel;
}

export interface BoardSelection {
  firmware: FirmwareVersion;
  refloat: RefloatVersion;
  topology: FixtureTopology;
  canId?: number;
  telemetry?: TelemetrySource;
  requestPolicy?: 'strict' | 'interactive';
  firmwareTestVersion?: number;
  shutdownBehavior?: ControllerShutdownBehavior;
}

export interface InteractiveControllerSelection {
  alias: string;
  canId: number;
  firmware: FirmwareVersion;
  refloat: RefloatVersion;
  uuid: Uint8Array;
}

export interface InteractiveBoardSelection {
  advertisedName: string;
  topology: FixtureTopology;
  controllers: readonly InteractiveControllerSelection[];
  canPingIds: readonly number[];
  telemetry: TelemetrySource;
  localBms?: BmsConfiguration;
  bmsNodes?: readonly BmsConfiguration[];
}

const expressFirmware = fixturePayload('../backends/fixtures/vesc/firmware-express-6.05.json');
const controllerFirmware = fixturePayload('../backends/fixtures/vesc/firmware-6.06.json');
const refloat = fixtureCases('../backends/fixtures/refloat/rtdata-1.2.3-android-hardware.json');
const rideStates = fixtureCases('../backends/fixtures/refloat/ride-states-1.2.3.json');

export const expressThor: BoardConfiguration = {
  name: 'Express / Thor300 / Refloat',
  advertisedName: 'FloatView Test VESC',
  transport: {
    major: 6,
    minor: 5,
    hardwareName: 'VESC Express T',
    hardwareType: 2,
    payload: expressFirmware,
  },
  controllers: [
    {
      canId: 1,
      firmware: {
        major: 6,
        minor: 6,
        hardwareName: 'THOR300',
        hardwareType: 0,
        payload: controllerFirmware,
      },
      refloatVersion: '1.2.3',
      refloatInfoPayload: required(refloat.get('versioned-info-v2'), 'Refloat info fixture'),
      telemetryLayoutPayload: required(refloat.get('dynamic-realtime-field-ids'), 'Refloat telemetry layout fixture'),
      refloatControls: new RefloatControlModel('1.2.3'),
      telemetryPhases: [
        phase('idle', 4),
        phase('riding', 12),
        phase('footpad-warning', 2),
        phase('post-ride-tail', 4),
        phase('return-to-idle', 2),
      ],
    },
  ],
};

export const expressTransport: FirmwareIdentity = expressThor.transport;

export const directThor: BoardConfiguration = {
  ...expressThor,
  name: 'Direct / Thor300 / Refloat',
  transport: expressThor.controllers[0]!.firmware,
};

export const directLegacyThor: BoardConfiguration = {
  ...directThor,
  name: 'Direct / Thor300 / Refloat legacy 1.0.0',
  controllers: [
    {
      ...directThor.controllers[0]!,
      refloatVersion: '1.0',
      refloatInfoPayload: refloatCodecs['1.0.0'].infoPayload,
      telemetryLayoutPayload: new Uint8Array(),
      refloatControls: new RefloatControlModel('1.0.0'),
      telemetryPhases: [
        {
          name: 'idle',
          repetitions: 4,
          payload: encodeRefloatTelemetry('1.0.0', idleTelemetry),
        },
      ],
    },
  ],
};

export const expressExplicitThor: BoardConfiguration = {
  ...expressThor,
  name: 'Express explicit CAN / Thor300 / Refloat',
  controllers: [{ ...expressThor.controllers[0]!, canId: 42 }],
  canPingIds: [],
};

export const genericGatewayThor: BoardConfiguration = {
  ...expressThor,
  name: 'Generic gateway / Thor300 / Refloat',
  transport: {
    major: 6,
    minor: 5,
    hardwareName: 'Generic VESC UART Gateway',
    hardwareType: 3,
    payload: encodeFirmwarePayload(
      {
        hardwareName: 'Generic VESC UART Gateway',
        hardwareType: 3,
        uuid: new Uint8Array(12),
      },
      6,
      5
    ),
  },
};

export const ambiguousThor: BoardConfiguration = {
  ...expressThor,
  name: 'Express / ambiguous controllers',
  controllers: [
    {
      ...expressThor.controllers[0]!,
      canId: 7,
      firmware: {
        major: 5,
        minor: 3,
        hardwareName: 'Unsupported VESC',
        hardwareType: 0,
        payload: encodeFirmwarePayload(
          {
            hardwareName: 'Unsupported VESC',
            hardwareType: 0,
            uuid: new Uint8Array(12).fill(7),
          },
          5,
          3
        ),
      },
    },
    {
      ...expressThor.controllers[0]!,
      canId: 42,
      firmware: {
        ...expressThor.controllers[0]!.firmware,
        payload: encodeFirmwarePayload(
          { ...thor300, uuid: new Uint8Array(12).fill(42) },
          6,
          6,
          Uint8Array.of(0, 0, 0, 0)
        ),
      },
    },
  ],
};

export function composeBoard(selection: BoardSelection): ComposedBoardConfiguration {
  const baseFirmware = firmwareVariants[selection.firmware];
  const testVersion = selection.firmwareTestVersion ?? baseFirmware.testVersion;
  const firmware =
    testVersion === baseFirmware.testVersion
      ? baseFirmware
      : {
          ...baseFirmware,
          testVersion,
          payload: encodeFirmwarePayload(
            thor300,
            baseFirmware.major,
            baseFirmware.minor,
            baseFirmware.hardwareCrc,
            testVersion
          ),
        };
  return {
    kind: 'composed',
    name: `${selection.topology === 'express' ? 'Express / ' : ''}THOR300 / Refloat ${selection.refloat}`,
    advertisedName: 'FloatView Test VESC',
    controller: thor300,
    firmware,
    refloat: refloatCodecs[selection.refloat],
    refloatControls: new RefloatControlModel(selection.refloat),
    shutdown: new ControllerShutdownModel(selection.shutdownBehavior),
    topology: selection.topology === 'express' ? expressTopology(selection.canId ?? 1) : directTopology,
    telemetry: selection.telemetry ?? {
      phase: () => 'idle',
      next: () => idleTelemetry,
    },
    requestPolicy: selection.requestPolicy ?? 'interactive',
    expressTransport,
  };
}

export function composeInteractiveBoard(selection: InteractiveBoardSelection): BoardConfiguration {
  if (selection.controllers.length === 0 || selection.controllers.length > 16) {
    throw new Error('Interactive board requires 1 through 16 controllers');
  }
  if (selection.topology === 'direct' && selection.controllers.length !== 1) {
    throw new Error('Direct topology requires exactly one controller');
  }
  const controllers = selection.controllers.map((item): ControllerConfiguration => {
    const firmware = firmwareVariants[item.firmware];
    const codec = refloatCodecs[item.refloat];
    return {
      canId: item.canId,
      firmware: {
        major: firmware.major,
        minor: firmware.minor,
        hardwareName: thor300.hardwareName,
        hardwareType: thor300.hardwareType,
        payload: encodeFirmwarePayload(
          { ...thor300, uuid: item.uuid },
          firmware.major,
          firmware.minor,
          firmware.hardwareCrc
        ),
      },
      refloatVersion: codec.version,
      refloatInfoPayload: codec.infoPayload,
      telemetryLayoutPayload: codec.layoutPayload ?? new Uint8Array(),
      telemetryPhases: [],
      refloat: codec,
      refloatControls: new RefloatControlModel(item.refloat),
      telemetry: selection.telemetry,
      shutdown: new ControllerShutdownModel(),
    };
  });
  return {
    name: `Workbench / ${selection.topology}`,
    advertisedName: selection.advertisedName,
    transport:
      selection.localBms?.firmware ?? (selection.topology === 'direct' ? controllers[0]!.firmware : expressTransport),
    controllers,
    ...(selection.localBms ? { localBms: selection.localBms } : {}),
    bmsNodes: [...(selection.bmsNodes ?? [])],
    canPingIds: [...selection.canPingIds],
    requestPolicy: 'interactive',
  };
}

function phase(name: TelemetryPhase['name'], repetitions: number): TelemetryPhase {
  return {
    name,
    repetitions,
    payload: required(rideStates.get(name), `Refloat ${name} fixture`),
  };
}

function fixturePayload(relativePath: string): Uint8Array {
  const value = fixtureJson(relativePath) as { frameHex?: string };
  if (!value.frameHex) throw new Error(`Fixture ${relativePath} has no frameHex`);
  return decodeFrame(value.frameHex);
}

function fixtureCases(relativePath: string): Map<string, Uint8Array> {
  const value = fixtureJson(relativePath) as {
    cases?: Array<{ name?: string; frameHex?: string }>;
  };
  return new Map(
    (value.cases ?? []).map((item) => {
      if (!item.name || !item.frameHex) throw new Error(`Invalid fixture case in ${relativePath}`);
      return [item.name, decodeFrame(item.frameHex)];
    })
  );
}

function fixtureJson(relativePath: string): unknown {
  const path = fileURLToPath(new URL(relativePath, new URL('../', import.meta.url)));
  return JSON.parse(readFileSync(path, 'utf8'));
}

function decodeFrame(hex: string): Uint8Array {
  const frames = new FrameAssembler().append(Uint8Array.from(Buffer.from(hex, 'hex')));
  if (frames.length !== 1 || !frames[0]) throw new Error('Fixture must contain exactly one VESC frame');
  return frames[0];
}

function required<T>(value: T | undefined, name: string): T {
  if (value === undefined) throw new Error(`${name} is missing`);
  return value;
}
