import { toHex } from "./bytes.js";

export type RequestKind =
  | "transportFirmware"
  | "canPing"
  | "primaryFirmware"
  | "refloatInfo"
  | "refloatTelemetryIds"
  | "refloatTelemetry";

export type RequestMatch =
  | { disposition: "readOnly"; kind: RequestKind; canId?: number }
  | { disposition: "mutating"; commandId: number; canId?: number; payloadHex: string }
  | { disposition: "unknown"; commandId: number; canId?: number; payloadHex: string };

// VESC commands that can affect motion, firmware, or persistent configuration.
// IDs follow the pinned VESC Tool datatypes.h command enumeration.
const MUTATING_COMMAND_IDS = new Set([
  5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 29, 30, 31, 32, 33, 37, 38, 39,
  40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57,
]);

export function matchRequest(payload: Uint8Array): RequestMatch {
  let body = payload;
  let canId: number | undefined;
  if (payload[0] === 34 && payload.length >= 3) {
    canId = payload[1]!;
    body = payload.slice(2);
  }
  const commandId = body[0] ?? 255;
  const withCan = <T extends object>(value: T): T & { canId?: number } =>
    canId === undefined ? value : { ...value, canId };

  if (body.length === 1 && commandId === 0) {
    return withCan({ disposition: "readOnly" as const, kind: canId === undefined ? "transportFirmware" : "primaryFirmware" });
  }
  if (canId === undefined && body.length === 1 && commandId === 62) {
    return { disposition: "readOnly", kind: "canPing" };
  }
  if (body.length === 5 && body[0] === 36 && body[1] === 101 && body[2] === 0 && body[3] === 2 && body[4] === 0) {
    return withCan({ disposition: "readOnly" as const, kind: "refloatInfo" as const });
  }
  if (body.length === 3 && body[0] === 36 && body[1] === 101 && body[2] === 32) {
    return withCan({ disposition: "readOnly" as const, kind: "refloatTelemetryIds" as const });
  }
  if (body.length === 3 && body[0] === 36 && body[1] === 101 && body[2] === 31) {
    return withCan({ disposition: "readOnly" as const, kind: "refloatTelemetry" as const });
  }
  return withCan({
    disposition: MUTATING_COMMAND_IDS.has(commandId) ? "mutating" as const : "unknown" as const,
    commandId,
    payloadHex: toHex(payload),
  });
}
