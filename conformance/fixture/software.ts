// Independent software-host protocol fixture. No production codec imports.
import { FrameAssembler, encodeFrame } from './frame.ts';
import { RefloatControlModel } from './refloat-controls.ts';
import { motorConfigurationPayload } from './firmware.ts';
import { refloatCodecs, type RefloatVersion } from './refloat.ts';
import type { TelemetrySource } from './telemetry.ts';
import { RefloatConfigurationModel } from './refloat-configuration.ts';
export class SoftwarePeripheral {
  private readonly telemetry?: TelemetrySource;
  private readonly refloatVersion: RefloatVersion;
  constructor(telemetry?: TelemetrySource, refloatVersion: RefloatVersion = '1.0.0') {
    this.telemetry = telemetry;
    this.refloatVersion = refloatVersion;
    this.configuration = new RefloatConfigurationModel(refloatVersion);
  }
  private frames = new FrameAssembler();
  private configuration: RefloatConfigurationModel;
  reset() {
    this.frames = new FrameAssembler();
    this.configuration.reset();
  }
  disconnect() {
    this.frames = new FrameAssembler();
  }
  fork(): SoftwarePeripheral {
    const copy = new SoftwarePeripheral(this.telemetry, this.refloatVersion);
    copy.frames = this.frames.fork();
    copy.configuration = this.configuration.fork();
    return copy;
  }
  receive(bytes: Uint8Array, consume?: (frame: Uint8Array) => void): Uint8Array[] {
    const responses: Uint8Array[] = [];
    for (const request of this.frames.append(bytes)) {
      const payload = this.respond(request);
      if (!payload) continue;
      const frame = encodeFrame(payload);
      if (consume) consume(frame);
      else responses.push(frame);
    }
    return responses;
  }
  private respond(request: Uint8Array): Uint8Array | undefined {
    const hex = (s: string) => new Uint8Array(Buffer.from(s, 'hex'));
    const same = (...expected: number[]) =>
      request.length === expected.length && request.every((v, i) => v === expected[i]);
    if (same(0))
      return Uint8Array.from([
        0,
        6,
        6,
        ...Buffer.from('THOR300\0'),
        ...new Array<number>(12).fill(0),
        0,
        0,
        0,
        1,
        0,
        0,
        0,
        0,
        ...Buffer.from('VESC\0'),
      ]);
    if (same(62)) return Uint8Array.of(62, 42);
    if (same(34, 42, 0)) return hex('00060056424d530000112233445566778899aabb00000101');
    if (same(34, 42, 96))
      return hex(
        '6003010b0003412140ff43a158ff445120000004e20000f424031068105e10540100010209f60a410abe09ab14820b6d036b03b62a428000004480000042000000440000002794014f4b00'
      );
    if (same(14) && this.refloatVersion !== '1.0.0') {
      const config = motorConfigurationPayload.slice();
      config[463] = 20;
      return config;
    }
    if (same(36, 101, 0, 2, 0)) return refloatCodecs[this.refloatVersion].infoPayload;
    if (request[0] === 36 && request[1] === 101 && request[2] === 35)
      return new RefloatControlModel(this.refloatVersion).alertsResponse(request);
    if (this.telemetry && request[0] === 36 && request[1] === 101 && request[2] !== 0) {
      return refloatCodecs[this.refloatVersion].respond(request, this.telemetry.next())?.payload;
    }
    if (same(36, 101, 10) || same(36, 101, 10, 4)) {
      const payload = new Uint8Array(55);
      payload.set([36, 101, 10, 3]);
      payload[10] = 1;
      payload[23] = 2;
      payload[24] = 4;
      payload[33] = 128;
      payload[39] = 60;
      payload[40] = 56;
      return payload;
    }
    return this.configuration.respond(request, true);
  }
}
