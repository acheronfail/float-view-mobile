import { firmwareFaultCode, type RefloatVersion } from './refloat.ts';
import type { TelemetrySample } from './telemetry.ts';

export type RefloatBooleanControl = 'package' | 'flywheel' | 'leds' | 'headlights';
export type RefloatProductControl = RefloatBooleanControl | 'lightingQuery' | 'clearFatalError';
export type RefloatControlFamily = 'legacy' | 'stable';

export interface RefloatControlProfile {
  version: RefloatVersion;
  family: RefloatControlFamily;
  fatalClearAvailable: boolean;
}

export interface RefloatControlRouteContext {
  selectedCanId?: number;
  requestCanId?: number;
  firmwareTestVersion?: number;
}

export interface DecodedRefloatControl {
  control: RefloatProductControl;
  enabled?: boolean;
  lightingMask?: number;
  lightingFlags?: number;
}

export type RefloatControlRejection = 'malformed' | 'wrong_route' | 'read_only' | 'unsupported_command';

export type RefloatControlDecodeResult =
  { accepted: true; value: DecodedRefloatControl } | { accepted: false; reason: RefloatControlRejection };

const profiles = new Map<RefloatVersion, RefloatControlProfile>([
  ['1.0.0', { version: '1.0.0', family: 'legacy', fatalClearAvailable: false }],
  ['1.0.1', { version: '1.0.1', family: 'legacy', fatalClearAvailable: false }],
  ['1.1.0', { version: '1.1.0', family: 'legacy', fatalClearAvailable: false }],
  ['1.1.1', { version: '1.1.1', family: 'legacy', fatalClearAvailable: false }],
  ['1.1.2', { version: '1.1.2', family: 'legacy', fatalClearAvailable: false }],
  ['1.2.0', { version: '1.2.0', family: 'stable', fatalClearAvailable: true }],
  ['1.2.1', { version: '1.2.1', family: 'stable', fatalClearAvailable: true }],
  ['1.2.2', { version: '1.2.2', family: 'stable', fatalClearAvailable: true }],
  ['1.2.3', { version: '1.2.3', family: 'stable', fatalClearAvailable: true }],
  ['1.3.0', { version: '1.3.0', family: 'stable', fatalClearAvailable: true }],
]);

export function refloatControlProfile(version: RefloatVersion): RefloatControlProfile {
  return profiles.get(version)!;
}

export function decodeRefloatProductControl(
  version: RefloatVersion,
  payload: Uint8Array,
  context: RefloatControlRouteContext = {}
): RefloatControlDecodeResult {
  if (
    context.selectedCanId !== undefined &&
    context.requestCanId !== undefined &&
    context.selectedCanId !== context.requestCanId
  ) {
    return { accepted: false, reason: 'wrong_route' };
  }
  if ((context.firmwareTestVersion ?? 0) !== 0) return { accepted: false, reason: 'read_only' };
  if (payload[0] !== 36 || payload[1] !== 101 || payload[2] === undefined) {
    return { accepted: false, reason: 'malformed' };
  }

  const profile = refloatControlProfile(version);
  switch (payload[2]) {
    case 12:
      if (payload.length !== 4 || (payload[3] !== 0 && payload[3] !== 1)) return malformed();
      return { accepted: true, value: { control: 'package', enabled: payload[3] === 0 } };
    case 22:
      if (
        payload.length !== 9 ||
        (payload[3] !== 0x80 && payload[3] !== 0x81) ||
        payload[4] !== 0 ||
        payload[5] !== 0 ||
        payload[6] !== 0 ||
        payload[7] !== 0 ||
        payload[8] !== 1
      ) {
        return malformed();
      }
      return { accepted: true, value: { control: 'flywheel', enabled: payload[3] === 0x81 } };
    case 202:
      if (profile.family !== 'legacy') return unsupported();
      if (payload.length !== 5 || !validLighting(payload[3], payload[4])) return malformed();
      return lighting(payload[3]!, payload[4]!);
    case 20:
      if (profile.family !== 'stable') return unsupported();
      if (
        payload.length !== 8 ||
        payload[3] !== 0 ||
        payload[4] !== 0 ||
        payload[5] !== 0 ||
        !validLighting(payload[6], payload[7])
      ) {
        return malformed();
      }
      return lighting(payload[6]!, payload[7]!);
    case 36:
      if (!profile.fatalClearAvailable) return unsupported();
      if (payload.length !== 4 || payload[3] !== 1) return malformed();
      return { accepted: true, value: { control: 'clearFatalError' } };
    default:
      return unsupported();
  }
}

function validLighting(mask: number | undefined, flags: number | undefined): boolean {
  return mask !== undefined && flags !== undefined && (mask & ~0x03) === 0 && (flags & ~0x03) === 0;
}

function lighting(mask: number, flags: number): RefloatControlDecodeResult {
  if (mask === 0) {
    return {
      accepted: true,
      value: { control: 'lightingQuery', lightingMask: mask, lightingFlags: flags },
    };
  }
  const control: RefloatBooleanControl = (mask & 0x02) !== 0 ? 'headlights' : 'leds';
  return {
    accepted: true,
    value: {
      control,
      enabled: (flags & (control === 'headlights' ? 0x02 : 0x01)) !== 0,
      lightingMask: mask,
      lightingFlags: flags,
    },
  };
}

function malformed(): RefloatControlDecodeResult {
  return { accepted: false, reason: 'malformed' };
}

function unsupported(): RefloatControlDecodeResult {
  return { accepted: false, reason: 'unsupported_command' };
}

export interface RefloatControlModelState {
  packageEnabled: boolean;
  flywheelEnabled: boolean;
  ledsEnabled: boolean;
  headlightsEnabled: boolean;
  fatalErrorActive: boolean;
  fatalErrorLatched: boolean;
}

export interface RefloatControlApplication {
  accepted: boolean;
  reason?: RefloatControlRejection | 'invalid_state';
  response?: Uint8Array;
  oneWay?: boolean;
}

/** Disposable source-faithful product-control state; it has no motor actuation path. */
export class RefloatControlModel {
  readonly version: RefloatVersion;
  readonly state: RefloatControlModelState;
  private faultCode = 0;
  private readonly clockTicks: () => number;
  private faultHistory: { ticks: number; active: boolean; code: number }[] = [];

  recordFirmwareFault(code: number) {
    if (!Number.isInteger(code) || code < 0 || code > 255) throw new Error('Invalid fault code');
    if (code === this.faultCode) return;
    this.faultCode = code;
    if (code) this.state.fatalErrorLatched = true;
    this.faultHistory.push({ ticks: this.clockTicks(), active: code !== 0, code });
    this.faultHistory = this.faultHistory.slice(-20);
  }

  alertsResponse(payload: Uint8Array): Uint8Array | undefined {
    if (!(this.version.startsWith('1.2.') || this.version === '1.3.0') || payload.length !== 7 || payload[2] !== 35)
      return;
    const since = new DataView(payload.buffer, payload.byteOffset, payload.byteLength).getUint32(3, false);
    const records = this.faultHistory.filter((r) => r.ticks > since);
    const bytes = [36, 101, 35, 0, 0, 0, this.faultCode ? 1 : 0, 0, 0, 0, 0, this.faultCode];
    if (this.faultCode) bytes.push(0); // Empty bounded name; app maps the raw code.
    bytes.push(records.length);
    for (const r of records) {
      bytes.push(
        (r.ticks >>> 24) & 255,
        (r.ticks >>> 16) & 255,
        (r.ticks >>> 8) & 255,
        r.ticks & 255,
        1,
        r.active ? 1 : 0,
        r.code
      );
      if (r.code) bytes.push(0);
    }
    return Uint8Array.from(bytes);
  }

  constructor(
    version: RefloatVersion,
    initial: Partial<RefloatControlModelState> = {},
    clockTicks = () => Math.floor(performance.now() * 10) >>> 0
  ) {
    this.clockTicks = clockTicks;
    this.version = version;
    this.state = {
      packageEnabled: true,
      flywheelEnabled: false,
      ledsEnabled: true,
      headlightsEnabled: true,
      fatalErrorActive: false,
      fatalErrorLatched: version.startsWith('1.2.') || version === '1.3.0',
      ...initial,
    };
  }

  apply(
    payload: Uint8Array,
    context: RefloatControlRouteContext & { rideState?: 'idle' | 'riding' | 'unknown' } = {}
  ): RefloatControlApplication {
    const decoded = decodeRefloatProductControl(this.version, payload, context);
    if (!decoded.accepted) return decoded;
    const value = decoded.value;
    if (value.control === 'lightingQuery') {
      const command = refloatControlProfile(this.version).family === 'legacy' ? 202 : 20;
      return {
        accepted: true,
        response: Uint8Array.of(
          36,
          101,
          command,
          (this.state.headlightsEnabled ? 2 : 0) | (this.state.ledsEnabled ? 1 : 0)
        ),
      };
    }
    const activeFlywheelDisable = this.state.flywheelEnabled && value.control === 'flywheel' && value.enabled === false;
    if (this.state.flywheelEnabled && !activeFlywheelDisable) {
      return { accepted: false, reason: 'invalid_state' };
    }
    if (
      (value.control === 'package' ||
        (value.control === 'flywheel' && !activeFlywheelDisable) ||
        value.control === 'clearFatalError') &&
      context.rideState !== undefined &&
      context.rideState !== 'idle'
    ) {
      return { accepted: false, reason: 'invalid_state' };
    }
    switch (value.control) {
      case 'package':
        this.state.packageEnabled = value.enabled!;
        return { accepted: true, oneWay: true };
      case 'flywheel':
        this.state.flywheelEnabled = value.enabled!;
        return { accepted: true, oneWay: true };
      case 'leds':
      case 'headlights': {
        const mask = value.lightingMask ?? 0;
        const flags = value.lightingFlags ?? 0;
        if (mask & 1) this.state.ledsEnabled = flags & 1 ? true : false;
        if (mask & 2) this.state.headlightsEnabled = flags & 2 ? true : false;
        const command = refloatControlProfile(this.version).family === 'legacy' ? 202 : 20;
        return {
          accepted: true,
          response: Uint8Array.of(
            36,
            101,
            command,
            (this.state.headlightsEnabled ? 2 : 0) | (this.state.ledsEnabled ? 1 : 0)
          ),
        };
      }
      case 'clearFatalError':
        if (this.state.fatalErrorActive || !this.state.fatalErrorLatched) {
          return { accepted: false, reason: 'invalid_state' };
        }
        this.state.fatalErrorLatched = false;
        return { accepted: true, oneWay: true };
    }
  }

  applyToSample(sample: TelemetrySample): TelemetrySample {
    const fault = this.state.fatalErrorActive ? (sample.firmwareFault ?? 'unknown') : sample.firmwareFault;
    this.recordFirmwareFault(fault === null ? 0 : firmwareFaultCode(fault));
    return {
      ...sample,
      packageState: this.state.flywheelEnabled
        ? 'running'
        : this.state.packageEnabled
          ? sample.packageState === 'disabled'
            ? 'ready'
            : sample.packageState
          : 'disabled',
      packageMode: this.state.flywheelEnabled
        ? 'flywheel'
        : sample.packageMode === 'flywheel'
          ? 'normal'
          : sample.packageMode,
      firmwareFault: this.state.fatalErrorActive ? (sample.firmwareFault ?? 'unknown') : sample.firmwareFault,
    };
  }

  decorateTelemetry(payload: Uint8Array): Uint8Array {
    if (this.version === '1.3.0' && payload[0] === 36 && payload[2] === 33 && payload.length >= 16) {
      const result = payload.slice();
      const view = new DataView(result.buffer);
      view.setUint32(12, this.clockTicks(), false);
      const mask = view.getUint32(4, false);
      if ((mask & 2) !== 0) {
        const offset = 16 + (mask & 1);
        if (result.length >= offset + 4) {
          const flags = view.getUint32(offset, false);
          view.setUint32(offset, this.state.fatalErrorLatched ? flags | (1 << 20) : flags & ~(1 << 20), false);
        }
      }
      return result;
    }
    if (!this.version.startsWith('1.2.') || payload[0] !== 36 || payload[2] !== 31 || payload.length < 9)
      return payload;
    const result = payload.slice();
    new DataView(result.buffer).setUint32(5, this.clockTicks(), false);
    result[4] = this.state.fatalErrorLatched ? (result[4] ?? 0) | 0x08 : (result[4] ?? 0) & ~0x08;
    return result;
  }
}
