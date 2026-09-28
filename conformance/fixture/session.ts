import { randomBytes } from 'node:crypto';
import { SoftwarePeripheral } from './software.ts';

export const LIMITS = { sessions: 4, commands: 64, events: 128, bytes: 65_536, message: 65_536, payload: 8_192 };
export type FixtureCommand = {
  run: string;
  session: string;
  operation: number;
  generation: number;
  command: 'scan' | 'stopScan' | 'connect' | 'disconnect' | 'write' | 'poll' | 'reset' | 'close';
  device?: string;
  bytes?: string;
};
export type FixtureEvent = {
  sequence: number;
  generation: number;
  operation: number;
  event: 'discovered' | 'link' | 'bytes';
  device: string;
  name?: string;
  rssi?: number;
  state?: 'ready' | 'disconnected';
  bytes?: string;
};
export type FixtureReply = Pick<FixtureCommand, 'run' | 'session' | 'operation' | 'generation'> & {
  events: FixtureEvent[];
};
const identity = () => randomBytes(24).toString('hex');
const integer = (v: unknown) => Number.isSafeInteger(v) && (v as number) >= 0;

export function checkEventCapacity(count: number, bytes: number) {
  if (count > LIMITS.events || bytes > LIMITS.bytes) throw new Error('Fixture event capacity exceeded');
}

export class FixtureSession {
  readonly id = identity();
  private model = new SoftwarePeripheral();
  private operation = 0;
  private generation = 0;
  private sequence = 0;
  private connected = false;
  private closed = false;
  private events: FixtureEvent[] = [];
  private byteCount = 0;
  readonly run: string;
  constructor(run: string) {
    this.run = run;
  }

  close() {
    this.closed = true;
    this.events = [];
    this.byteCount = 0;
    this.model.reset();
  }

  execute(input: FixtureCommand): FixtureReply {
    if (input.run !== this.run || input.session !== this.id) throw new Error('Foreign fixture session');
    if (!integer(input.operation) || input.operation !== this.operation + 1)
      throw new Error('Unknown or duplicate operation');
    if (!integer(input.generation)) throw new Error('Invalid generation');
    if (input.command === 'close') {
      this.close();
      return { ...this.correlation(input), events: [] };
    }
    if (this.closed) throw new Error('Fixture session closed');
    // Validate before committing either protocol state or operation identity.
    const connecting = input.command === 'connect';
    if (input.generation !== this.generation + (connecting ? 1 : 0)) throw new Error('Stale generation');
    const next = this.model.fork();
    const events: FixtureEvent[] = [];
    let stagedBytes = 0;
    const replacing = connecting || input.command === 'disconnect' || input.command === 'reset';
    const emit = (event: Omit<FixtureEvent, 'sequence' | 'operation' | 'generation' | 'device'>) => {
      stagedBytes += event.bytes ? Buffer.byteLength(event.bytes, 'base64') : 0;
      try {
        checkEventCapacity(
          (replacing ? 0 : this.events.length) + events.length + 1,
          (replacing ? 0 : this.byteCount) + stagedBytes
        );
      } catch (error) {
        this.close();
        throw error;
      }
      events.push({
        ...event,
        sequence: this.sequence + events.length + 1,
        operation: input.operation,
        generation: input.generation,
        device: 'software-vesc',
      });
    };
    switch (input.command) {
      case 'scan':
        emit({ event: 'discovered', name: 'Software VESC', rssi: -42 });
        break;
      case 'stopScan':
        break;
      case 'connect':
        if (input.device !== 'software-vesc') throw new Error('Unknown device');
        next.disconnect();
        emit({ event: 'link', state: 'ready' });
        break;
      case 'disconnect':
        next.disconnect();
        emit({ event: 'link', state: 'disconnected' });
        break;
      case 'write': {
        if (!this.connected || input.device !== 'software-vesc') throw new Error('No fixture connection');
        try {
          const inputBytes = decodeBytes(input.bytes);
          next.receive(inputBytes, (bytes) => emit({ event: 'bytes', bytes: Buffer.from(bytes).toString('base64') }));
        } catch (error) {
          this.close();
          throw error;
        }
        break;
      }
      case 'reset':
        next.reset();
        break;
      case 'poll':
        break;
      default:
        throw new Error('Unknown fixture command');
    }
    this.model = next;
    this.operation = input.operation;
    this.generation = input.generation;
    this.sequence += events.length;
    if (replacing) {
      // Undelivered traffic never crosses a connection boundary.
      this.events = [];
      this.byteCount = 0;
      if (connecting || input.command === 'disconnect') this.connected = connecting;
    }
    this.events.push(...events);
    this.byteCount += stagedBytes;
    return this.drain(input);
  }

  private correlation(input: FixtureCommand) {
    return { run: this.run, session: this.id, operation: input.operation, generation: input.generation };
  }
  private drain(input: FixtureCommand): FixtureReply {
    const reply: FixtureReply = { ...this.correlation(input), events: [] };
    // One reply may be smaller than the bounded queue (base64 expands bytes).
    while (this.events.length) {
      const event = this.events[0]!;
      reply.events.push(event);
      if (Buffer.byteLength(JSON.stringify(reply)) > LIMITS.message) {
        reply.events.pop();
        break;
      }
      this.events.shift();
      this.byteCount -= event.bytes ? Buffer.byteLength(event.bytes, 'base64') : 0;
    }
    return reply;
  }
}

export function decodeBytes(value: unknown): Uint8Array {
  if (
    typeof value !== 'string' ||
    value.length > Math.ceil(LIMITS.payload / 3) * 4 ||
    !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value) ||
    Buffer.byteLength(value, 'base64') > LIMITS.payload
  )
    throw new Error('Invalid bounded byte payload');
  return Buffer.from(value, 'base64');
}

export class FixtureRun {
  readonly id = identity();
  readonly token = identity();
  private sessions = new Map<string, FixtureSession>();
  create() {
    if (this.sessions.size >= LIMITS.sessions) throw new Error('Fixture session capacity exceeded');
    const session = new FixtureSession(this.id);
    this.sessions.set(session.id, session);
    return session;
  }
  has(id: string) {
    return this.sessions.has(id);
  }
  fail(id: string) {
    this.sessions.get(id)?.close();
  }
  execute(input: FixtureCommand) {
    const session = this.sessions.get(input.session);
    if (!session) throw new Error('Unknown fixture session');
    const reply = session.execute(input);
    if (input.command === 'close') this.sessions.delete(session.id);
    return reply;
  }
  close() {
    for (const session of this.sessions.values()) session.close();
    this.sessions.clear();
  }
}
