// Independent software-host protocol fixture. No production codec imports.
import { FrameAssembler, encodeFrame } from './frame.ts';
import { refloatCodecs } from './refloat.ts';
import { RefloatControlModel } from './refloat-controls.ts';
import { idleTelemetry } from './telemetry.ts';
import { RefloatConfigurationModel } from './refloat-configuration.ts';
export class SoftwarePeripheral {
  private frames = new FrameAssembler();
  private beta = process.env.FLOAT_VIEW_FIXTURE_REFLOAT === '1.3.1-beta1';
  private configuration = new RefloatConfigurationModel(this.beta ? '1.3.1-beta1' : '1.0.0');
  private controls = new RefloatControlModel('1.3.1-beta1', { fatalErrorLatched: false });
  constructor() {
    if (this.beta) {
      this.controls.recordImuTimeout(true);
      this.controls.recordImuTimeout(false);
    }
  }
  reset() {
    this.frames = new FrameAssembler();
    this.configuration.reset();
  }
  disconnect() {
    this.frames = new FrameAssembler();
  }
  fork(): SoftwarePeripheral {
    const copy = Object.create(SoftwarePeripheral.prototype) as SoftwarePeripheral;
    copy.frames = this.frames.fork();
    copy.configuration = this.configuration.fork();
    copy.beta = this.beta;
    copy.controls = this.controls.fork();
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
    if (this.beta) {
      if (same(36, 101, 0, 2, 0)) return refloatCodecs['1.3.1-beta1'].infoPayload;
      if (request[0] === 36 && request[1] === 101) {
        if (request[2] === 35) return this.controls.alertsResponse(request);
        const telemetry = refloatCodecs['1.3.1-beta1'].respond(request, this.controls.applyToSample(idleTelemetry));
        if (telemetry) return this.controls.decorateTelemetry(telemetry.payload);
        const control = this.controls.apply(request, { rideState: 'idle' });
        return control.accepted ? control.response : undefined;
      }
      return this.configuration.respond(request, true);
    }
    if (same(36, 101, 0, 2, 0)) return Uint8Array.of(36, 101, 0, 10);
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
