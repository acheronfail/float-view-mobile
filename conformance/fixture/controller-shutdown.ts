export type ControllerShutdownBehavior = 'disconnect' | 'no-op';
export type ControllerShutdownRejection =
  | 'malformed'
  | 'read_only'
  | 'unsupported_version'
  | 'invalid_state'
  | 'unsafe_variant'
  | 'unsupported_command'
  | 'wrong_route';

export interface ControllerShutdownContext {
  rideState: 'idle' | 'riding' | 'unknown';
  flywheel: boolean;
  mutationPending: boolean;
}

export interface ControllerShutdownDecodeContext {
  selectedCanId?: number;
  firmwareTestVersion?: number;
  shutdownPending?: boolean;
}

export function decodeControllerShutdown(
  version: string,
  request: Uint8Array,
  context: ControllerShutdownDecodeContext = {}
): { accepted: true } | { accepted: false; reason: ControllerShutdownRejection } {
  if (!['6.05', '6.06', '7.00'].includes(version)) {
    return { accepted: false, reason: 'unsupported_version' };
  }
  if (context.shutdownPending) return { accepted: false, reason: 'invalid_state' };

  let payload = request;
  if (request[0] === 34) {
    if (request.length !== 5 || request[1] === 255 || request[1] !== context.selectedCanId) {
      return { accepted: false, reason: 'wrong_route' };
    }
    payload = request.slice(2);
  } else if (context.selectedCanId !== undefined) {
    return { accepted: false, reason: 'wrong_route' };
  }
  if (payload[0] !== 156) return { accepted: false, reason: 'unsupported_command' };
  if (payload.length !== 3) return { accepted: false, reason: 'malformed' };
  if (payload[1] !== 0 || payload[2] !== 0) return { accepted: false, reason: 'unsafe_variant' };
  return { accepted: true };
}

export class ControllerShutdownModel {
  readonly behavior: ControllerShutdownBehavior;
  attempted = false;
  available = true;

  constructor(behavior: ControllerShutdownBehavior = 'disconnect') {
    this.behavior = behavior;
  }

  apply(
    firmware: { version: string; testVersion: number },
    payload: Uint8Array,
    context: ControllerShutdownContext
  ): { accepted: true; disconnected: boolean } | { accepted: false; reason: ControllerShutdownRejection } {
    const decoded = decodeControllerShutdown(firmware.version, payload, {
      firmwareTestVersion: firmware.testVersion,
      shutdownPending: this.attempted,
    });
    if (!decoded.accepted) return decoded;
    if (context.rideState !== 'idle' || context.flywheel || context.mutationPending) {
      return { accepted: false, reason: 'invalid_state' };
    }
    this.attempted = true;
    const disconnected = this.behavior === 'disconnect';
    if (disconnected) this.available = false;
    return { accepted: true, disconnected };
  }
}
