import { concatBytes } from "./bytes.js";

export const MAXIMUM_PAYLOAD_BYTES = 4096;

export function crc16(payload: Uint8Array): number {
  let crc = 0;
  for (const byte of payload) {
    crc ^= byte << 8;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc & 0x8000) !== 0 ? ((crc << 1) ^ 0x1021) & 0xffff : (crc << 1) & 0xffff;
    }
  }
  return crc;
}

export function encodeFrame(payload: Uint8Array): Uint8Array {
  if (payload.length > MAXIMUM_PAYLOAD_BYTES) {
    throw new Error(`VESC payload exceeds ${MAXIMUM_PAYLOAD_BYTES} bytes`);
  }
  const header =
    payload.length <= 255
      ? Uint8Array.of(2, payload.length)
      : Uint8Array.of(3, payload.length >> 8, payload.length & 0xff);
  const crc = crc16(payload);
  return concatBytes(header, payload, Uint8Array.of(crc >> 8, crc & 0xff, 3));
}

export function decodeFrame(frame: Uint8Array): Uint8Array {
  const start = frame[0];
  if (start !== 2 && start !== 3) throw new Error("invalid VESC frame start");
  const headerLength = start === 2 ? 2 : 3;
  if (frame.length < headerLength + 3) throw new Error("truncated VESC frame");
  const payloadLength = start === 2 ? frame[1]! : (frame[1]! << 8) | frame[2]!;
  if (payloadLength > MAXIMUM_PAYLOAD_BYTES) throw new Error("VESC payload is too large");
  if (frame.length !== headerLength + payloadLength + 3 || frame.at(-1) !== 3) {
    throw new Error("invalid VESC frame length or terminator");
  }
  const payload = frame.slice(headerLength, headerLength + payloadLength);
  const received = (frame[headerLength + payloadLength]! << 8) | frame[headerLength + payloadLength + 1]!;
  if (crc16(payload) !== received) throw new Error("invalid VESC CRC");
  return payload;
}

export type StreamResult =
  | { kind: "frame"; frame: Uint8Array; payload: Uint8Array }
  | { kind: "fault"; code: "invalid_crc" | "invalid_frame" | "payload_too_large"; message: string };

export class VescStreamAssembler {
  private buffer: Uint8Array<ArrayBufferLike> = new Uint8Array();

  constructor(private readonly maximumBufferedBytes = MAXIMUM_PAYLOAD_BYTES * 2) {}

  append(chunk: Uint8Array): StreamResult[] {
    this.buffer = concatBytes(this.buffer, chunk);
    const results: StreamResult[] = [];
    if (this.buffer.length > this.maximumBufferedBytes) {
      this.buffer = new Uint8Array();
      return [{ kind: "fault", code: "payload_too_large", message: "VESC stream buffer limit exceeded" }];
    }
    while (this.buffer.length > 0) {
      const start = this.buffer.findIndex((byte) => byte === 2 || byte === 3);
      if (start < 0) {
        this.buffer = new Uint8Array();
        break;
      }
      if (start > 0) this.buffer = this.buffer.slice(start);
      const marker = this.buffer[0]!;
      const headerLength = marker === 2 ? 2 : 3;
      if (this.buffer.length < headerLength) break;
      const payloadLength =
        marker === 2 ? this.buffer[1]! : (this.buffer[1]! << 8) | this.buffer[2]!;
      if (payloadLength > MAXIMUM_PAYLOAD_BYTES) {
        this.buffer = this.buffer.slice(1);
        results.push({ kind: "fault", code: "payload_too_large", message: "VESC frame declared an oversized payload" });
        continue;
      }
      const frameLength = headerLength + payloadLength + 3;
      if (this.buffer.length < frameLength) break;
      const frame = this.buffer.slice(0, frameLength);
      this.buffer = this.buffer.slice(frameLength);
      try {
        results.push({ kind: "frame", frame, payload: decodeFrame(frame) });
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        results.push({
          kind: "fault",
          code: message.includes("CRC") ? "invalid_crc" : "invalid_frame",
          message,
        });
      }
    }
    return results;
  }

  reset(): void {
    this.buffer = new Uint8Array();
  }
}
