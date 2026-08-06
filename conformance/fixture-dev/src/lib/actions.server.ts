import { FixtureActionError, type FixtureCoordinator } from './coordinator/coordinator.server.ts';
import {
  bmsTopologyOptions,
  firmwareOptions,
  refloatOptions,
  topologyOptions,
  type FixtureAction,
  type FixtureConfiguration,
} from './contract.ts';
import { bmsFailureModes } from '../../../fixture/bms.ts';
import { telemetryFieldNames, telemetryScenes } from '../../../fixture/telemetry.ts';
import { parseScenarioDocument } from './coordinator/scenario-document.server.ts';
import { authorizeMutation, HttpActionError } from './auth.server.ts';

export async function handleActionRequest(
  request: Request,
  coordinator: FixtureCoordinator,
  token?: string
): Promise<Response> {
  try {
    authorizeMutation(request, token);
    const text = await request.text();
    if (new TextEncoder().encode(text).length > 65_536) {
      throw new HttpActionError(413, 'Fixture action body exceeds bound');
    }
    const action = parseAction(JSON.parse(text) as unknown);
    const snapshot = await coordinator.act(action);
    return json(snapshot);
  } catch (error) {
    if (error instanceof FixtureActionError) {
      return json({ error: error.message, snapshot: error.snapshot }, 409);
    }
    if (error instanceof HttpActionError) return json({ error: error.message }, error.status);
    return json({ error: message(error) }, error instanceof SyntaxError ? 400 : 400);
  }
}

export function parseAction(value: unknown): FixtureAction {
  if (!isRecord(value) || typeof value.kind !== 'string') throw new Error('Invalid fixture action');
  switch (value.kind) {
    case 'attach':
      if (typeof value.path !== 'string' || !value.path || value.path.length > 512) {
        throw new Error('Invalid serial path');
      }
      return { kind: 'attach', path: value.path };
    case 'detach':
    case 'power-on':
    case 'power-off':
    case 'start-riding':
    case 'stop-riding':
      return { kind: value.kind };
    case 'apply-template':
      if (
        value.template !== 'direct' &&
        value.template !== 'singleExpress' &&
        value.template !== 'reportedMultiController' &&
        value.template !== 'directVbms'
      ) {
        throw new Error('Invalid scenario template');
      }
      return { kind: value.kind, template: value.template };
    case 'import-scenario':
      return { kind: value.kind, document: parseScenarioDocument(value.document) };
    case 'set-scene':
      if (typeof value.scene !== 'string' || !telemetryScenes.includes(value.scene as never)) {
        throw new Error('Invalid telemetry scene');
      }
      return { kind: value.kind, scene: value.scene as (typeof telemetryScenes)[number] };
    case 'freeze':
    case 'resume':
    case 'reset-overrides':
    case 'disconnect-readvertise':
    case 'reconnect-unchanged':
      return { kind: value.kind };
    case 'pin-field':
      if (
        typeof value.field !== 'string' ||
        !telemetryFieldNames.includes(value.field as never) ||
        !('value' in value)
      ) {
        throw new Error('Invalid field pin');
      }
      return {
        kind: value.kind,
        field: value.field as never,
        value: value.value as never,
      };
    case 'unpin-field':
      if (typeof value.field !== 'string' || !telemetryFieldNames.includes(value.field as never)) {
        throw new Error('Invalid field pin');
      }
      return { kind: value.kind, field: value.field as never };
    case 'move-controller':
      if (typeof value.alias !== 'string' || !Number.isInteger(value.canId)) {
        throw new Error('Invalid controller move');
      }
      return { kind: value.kind, alias: value.alias, canId: Number(value.canId) };
    case 'replace-controller':
      if (typeof value.alias !== 'string') throw new Error('Invalid controller replacement');
      return { kind: value.kind, alias: value.alias };
    case 'set-ping-visible':
      if (typeof value.alias !== 'string' || typeof value.visible !== 'boolean') {
        throw new Error('Invalid ping visibility');
      }
      return { kind: value.kind, alias: value.alias, visible: value.visible };
    case 'set-fragment-bytes':
      if (!Number.isInteger(value.fragmentBytes)) throw new Error('Invalid fragment bound');
      return { kind: value.kind, fragmentBytes: Number(value.fragmentBytes) };
    case 'configure': {
      const configuration = value.configuration;
      if (!isRecord(configuration)) throw new Error('Invalid fixture configuration - no configuration');
      if (
        typeof configuration.topology !== 'string' ||
        !(topologyOptions as readonly string[]).includes(configuration.topology)
      ) {
        throw new Error('Invalid fixture configuration - missing topology');
      }
      if (
        typeof configuration.firmware !== 'string' ||
        !(firmwareOptions as readonly string[]).includes(configuration.firmware)
      ) {
        throw new Error('Invalid fixture configuration - missing firmware');
      }
      if (
        typeof configuration.refloat !== 'string' ||
        !(refloatOptions as readonly string[]).includes(configuration.refloat)
      ) {
        throw new Error('Invalid fixture configuration - missing refloat');
      }
      if (
        configuration.topology !== 'direct' &&
        (typeof configuration.canId !== 'string' || !/^(0|[1-9][0-9]{0,2})$/.test(configuration.canId))
      ) {
        throw new Error('Invalid fixture configuration - missing canId');
      }
      const canId = Number(configuration.canId);
      if (canId > 253) throw new Error('CAN ID must be from 0 through 253');
      if (
        typeof configuration.bmsTopology !== 'string' ||
        !(bmsTopologyOptions as readonly string[]).includes(configuration.bmsTopology)
      ) {
        throw new Error('Invalid fixture configuration - missing BMS topology');
      }
      const bmsCanId = canonicalInteger(configuration.bmsCanId, 'BMS CAN ID', 0, 253);
      if (configuration.bmsVariant !== 'legacy' && configuration.bmsVariant !== 'extended') {
        throw new Error('Invalid fixture configuration - missing BMS variant');
      }
      if (
        typeof configuration.bmsFailureMode !== 'string' ||
        !bmsFailureModes.includes(configuration.bmsFailureMode as never)
      ) {
        throw new Error('Invalid fixture configuration - missing BMS failure mode');
      }
      const bmsCellCount = canonicalInteger(configuration.bmsCellCount, 'BMS cell count', 1, 50);
      const bmsTemperatureCount = canonicalInteger(configuration.bmsTemperatureCount, 'BMS temperature count', 0, 50);
      const bmsStateOfCharge = canonicalDecimal(configuration.bmsStateOfCharge, 'BMS state of charge', 0, 1);
      const bmsInputCurrentA = canonicalDecimal(configuration.bmsInputCurrentA, 'BMS input current', -200, 200);
      return {
        kind: 'configure',
        configuration: {
          topology: configuration.topology as FixtureConfiguration['topology'],
          firmware: configuration.firmware as FixtureConfiguration['firmware'],
          refloat: configuration.refloat as FixtureConfiguration['refloat'],
          canId,
          bmsTopology: configuration.bmsTopology as FixtureConfiguration['bmsTopology'],
          bmsCanId,
          bmsVariant: configuration.bmsVariant as FixtureConfiguration['bmsVariant'],
          bmsFailureMode: configuration.bmsFailureMode as FixtureConfiguration['bmsFailureMode'],
          bmsCellCount,
          bmsTemperatureCount,
          bmsStateOfCharge,
          bmsInputCurrentA,
        },
      };
    }
    default:
      throw new Error('Unknown fixture action');
  }
}

function canonicalInteger(value: unknown, label: string, minimum: number, maximum: number) {
  if (typeof value !== 'string' || !/^(0|[1-9][0-9]*)$/.test(value)) {
    throw new Error(`Invalid fixture configuration - missing ${label}`);
  }
  const parsed = Number(value);
  if (parsed < minimum || parsed > maximum) throw new Error(`${label} must be from ${minimum} through ${maximum}`);
  return parsed;
}

function canonicalDecimal(value: unknown, label: string, minimum: number, maximum: number) {
  if (typeof value !== 'string' || !/^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?$/.test(value)) {
    throw new Error(`Invalid fixture configuration - missing ${label}`);
  }
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed < minimum || parsed > maximum) {
    throw new Error(`${label} must be from ${minimum} through ${maximum}`);
  }
  return parsed;
}

function json(value: unknown, status = 200) {
  return Response.json(value, {
    status,
    headers: {
      'cache-control': 'no-store',
      'content-security-policy': "default-src 'none'; frame-ancestors 'none'",
    },
  });
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function message(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
