export const MAX_PAYLOAD_BYTES = 4_096;

export function crc16(bytes: Uint8Array) {
  let crc = 0;
  for (const byte of bytes) {
    crc ^= byte << 8;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc & 0x8000) !== 0 ? ((crc << 1) ^ 0x1021) & 0xffff : (crc << 1) & 0xffff;
    }
  }
  return crc;
}

export function encodeFrame(payload: Uint8Array) {
  if (payload.length > MAX_PAYLOAD_BYTES) throw new Error('VESC payload exceeds bound');
  const header =
    payload.length <= 255
      ? Uint8Array.of(2, payload.length)
      : Uint8Array.of(3, payload.length >> 8, payload.length & 0xff);
  const crc = crc16(payload);
  return Uint8Array.from([...header, ...payload, crc >> 8, crc & 0xff, 3]);
}

export class FrameAssembler {
  private bytes: number[] = [];

  append(chunk: Uint8Array) {
    this.bytes.push(...chunk);
    if (this.bytes.length > MAX_PAYLOAD_BYTES * 2) {
      this.bytes = [];
      throw new Error('VESC stream exceeds bound');
    }
    const payloads: Uint8Array[] = [];
    while (this.bytes.length) {
      while (this.bytes.length && this.bytes[0] !== 2 && this.bytes[0] !== 3) this.bytes.shift();
      if (!this.bytes.length) break;
      const header = this.bytes[0] === 2 ? 2 : 3;
      if (this.bytes.length < header) break;
      const length = header === 2 ? this.bytes[1]! : (this.bytes[1]! << 8) | this.bytes[2]!;
      if (length > MAX_PAYLOAD_BYTES) throw new Error('VESC payload exceeds bound');
      const total = header + length + 3;
      if (this.bytes.length < total) break;
      const frame = this.bytes.splice(0, total);
      if (frame.at(-1) !== 3) throw new Error('Invalid VESC frame terminator');
      const payload = Uint8Array.from(frame.slice(header, header + length));
      const expected = (frame[header + length]! << 8) | frame[header + length + 1]!;
      if (crc16(payload) !== expected) throw new Error('Invalid VESC frame CRC');
      payloads.push(payload);
    }
    return payloads;
  }
}
