export type FirmwareVersion = '6.02' | '6.05' | '6.06' | '7.00';

export interface ControllerIdentity {
  hardwareName: string;
  hardwareType: number;
  uuid: Uint8Array;
}

export interface FirmwareVariant {
  version: FirmwareVersion;
  major: number;
  minor: number;
  hardwareCrc?: Uint8Array;
  testVersion: number;
  payload: Uint8Array;
  provenance: string;
}

export const thor300: ControllerIdentity = {
  hardwareName: 'THOR300',
  hardwareType: 0,
  uuid: Uint8Array.from({ length: 12 }, (_, index) => index + 1),
};

/** VESC 7.00 COMM_GET_MCCONF with current limits and an 18S battery setup. */
export const motorConfigurationPayload = (() => {
  const payload = new Uint8Array(489);
  payload.set(
    [14, 0x57, 0xad, 0x8f, 0x53, 0, 0, 0, 0, 0x42, 0xc8, 0, 0, 0xc2, 0xc8, 0, 0, 0x42, 0x48, 0, 0, 0xc1, 0xa0, 0, 0],
    0
  );
  payload[463] = 18;
  return payload;
})();

/** Independent source-derived widths/positions from pinned VESC Tool SerOrder. */
export function motorConfigurationForVersion(
  major: number,
  minor: number
): { bytes: Uint8Array; diameterOffset: number } | null {
  const layout = (
    {
      '6.2': [0x2e43a161, 482, 457],
      '6.5': [0x3f829cf7, 478, 447],
      '6.6': [0x2efd0142, 484, 453],
      '7.0': [0x57ad8f53, 489, 458],
    } as Record<string, number[]>
  )[`${major}.${minor}`];
  if (!layout) return null;
  const [signature, length, diameterOffset] = layout as [number, number, number];
  const bytes = new Uint8Array(length);
  bytes.set(motorConfigurationPayload.subarray(0, 25));
  const view = new DataView(bytes.buffer);
  view.setUint32(1, signature, false);
  bytes[diameterOffset - 5] = 30;
  view.setFloat32(diameterOffset - 4, 1, false);
  view.setFloat32(diameterOffset, 0.28, false);
  bytes[diameterOffset + 5] = 18;
  return { bytes, diameterOffset };
}

/** Strict fixture allowlist: reject every motor mutation except a diameter-only patch. */
export function applyWheelDiameterWrite(
  current: Uint8Array,
  request: Uint8Array,
  pos: number,
  safe: boolean
): Uint8Array | null {
  const meters =
    request.length >= pos + 4
      ? new DataView(request.buffer, request.byteOffset, request.byteLength).getFloat32(pos, false)
      : NaN;
  if (
    request[0] !== 13 ||
    !safe ||
    request.length !== current.length ||
    !Number.isFinite(meters) ||
    meters < Math.fround(0.1) ||
    meters > Math.fround(0.6) ||
    request.some((b, i) => i > 0 && (i < pos || i >= pos + 4) && b !== current[i])
  )
    return null;
  const next = request.slice();
  next[0] = 14;
  return next;
}

const provenance =
  'VESC COMM_FW_VERSION field ordering derived from vedderb/bldc and vedderb/vesc_fw_archive version changelogs; THOR300 identity is deliberate fixture data';

export const firmwareVariants: Readonly<Record<FirmwareVersion, FirmwareVariant>> = {
  '6.02': variant(6, 2),
  '6.05': variant(6, 5, Uint8Array.of(0, 0, 0, 0)),
  '6.06': variant(6, 6, Uint8Array.of(0, 0, 0, 0)),
  '7.00': variant(7, 0, Uint8Array.of(0, 0, 0, 0)),
};

function variant(major: number, minor: number, hardwareCrc?: Uint8Array, testVersion = 0): FirmwareVariant {
  const version = `${major}.${minor.toString().padStart(2, '0')}` as FirmwareVersion;
  return {
    version,
    major,
    minor,
    hardwareCrc,
    testVersion,
    payload: encodeFirmwarePayload(thor300, major, minor, hardwareCrc, testVersion),
    provenance,
  };
}

export function encodeFirmwarePayload(
  identity: ControllerIdentity,
  major: number,
  minor: number,
  hardwareCrc?: Uint8Array,
  testVersion = 0,
  customConfigurationCount = 0
): Uint8Array {
  const name = new TextEncoder().encode(identity.hardwareName);
  const commonAndCapabilities = Uint8Array.of(
    0,
    testVersion,
    identity.hardwareType,
    customConfigurationCount,
    0,
    0,
    0,
    0,
    0
  );
  return Uint8Array.from([
    0,
    major,
    minor,
    ...name,
    0,
    ...identity.uuid,
    ...commonAndCapabilities,
    ...(hardwareCrc ?? []),
  ]);
}
