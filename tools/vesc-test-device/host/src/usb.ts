import { EventEmitter } from "node:events";

export interface ControlEvent {
  protocolVersion: number;
  sequence: number;
  kind: string;
  commandSequence?: number;
  commandKind?: string;
  status?: string;
  code?: string;
  message?: string;
  [key: string]: unknown;
}

export interface LineTransport {
  write(line: string): Promise<void>;
  onLine(handler: (line: string) => void): () => void;
  close(): Promise<void>;
}

export interface UsbClock {
  setTimeout(handler: () => void, milliseconds: number): unknown;
  clearTimeout(handle: unknown): void;
}

const systemClock: UsbClock = {
  setTimeout: (handler, milliseconds) => setTimeout(handler, milliseconds),
  clearTimeout: (handle) => clearTimeout(handle as NodeJS.Timeout),
};

export class UsbControlClient extends EventEmitter {
  private hostSequence = 0;
  private lastDeviceSequence = 0;
  private readonly pending = new Map<
    number,
    {
      kind: string;
      resolve: (event: ControlEvent) => void;
      reject: (error: Error) => void;
      timer: unknown;
    }
  >();
  private readonly unsubscribe: () => void;

  constructor(
    private readonly transport: LineTransport,
    private readonly timeoutMs = 3_000,
    private readonly clock: UsbClock = systemClock,
    initialHostSequence = 0,
  ) {
    super();
    this.hostSequence = initialHostSequence;
    this.unsubscribe = transport.onLine((line) => this.receiveLine(line));
  }

  async command(
    kind: string,
    fields: Record<string, unknown> = {},
  ): Promise<ControlEvent> {
    const sequence = ++this.hostSequence;
    const body = JSON.stringify({
      protocolVersion: 1,
      sequence,
      kind,
      ...fields,
    });
    const completion = new Promise<ControlEvent>((resolve, reject) => {
      const timer = this.clock.setTimeout(() => {
        this.pending.delete(sequence);
        reject(
          new Error(
            `USB command ${kind} sequence ${sequence} timed out after ${this.timeoutMs}ms`,
          ),
        );
      }, this.timeoutMs);
      this.pending.set(sequence, { kind, resolve, reject, timer });
    });
    try {
      await this.transport.write(`${body}\n`);
    } catch (error) {
      const pending = this.pending.get(sequence);
      if (pending) {
        this.clock.clearTimeout(pending.timer);
        this.pending.delete(sequence);
        pending.reject(
          error instanceof Error ? error : new Error(String(error)),
        );
      }
    }
    return completion;
  }

  async close(): Promise<void> {
    this.unsubscribe();
    for (const [sequence, pending] of this.pending) {
      this.clock.clearTimeout(pending.timer);
      pending.reject(
        new Error(
          `USB transport closed with command ${pending.kind} sequence ${sequence} pending`,
        ),
      );
    }
    this.pending.clear();
    await this.transport.close();
  }

  private receiveLine(line: string): void {
    let event: ControlEvent;
    try {
      const parsed = JSON.parse(line) as Partial<ControlEvent>;
      if (
        parsed.protocolVersion !== 1 ||
        typeof parsed.sequence !== "number" ||
        typeof parsed.kind !== "string"
      )
        return;
      event = parsed as ControlEvent;
    } catch {
      this.emit("parseError", line);
      return;
    }
    if (event.kind === "boot") {
      this.lastDeviceSequence = event.sequence;
      if (this.pending.size > 0) {
        for (const [sequence, pending] of this.pending) {
          this.clock.clearTimeout(pending.timer);
          pending.reject(
            new Error(
              `ESP reset while command ${pending.kind} sequence ${sequence} was pending`,
            ),
          );
        }
        this.pending.clear();
      }
      this.emit("event", event);
      return;
    }
    if (event.sequence <= this.lastDeviceSequence) {
      this.emit("staleEvent", event);
      return;
    }
    this.lastDeviceSequence = event.sequence;
    const commandSequence = event.commandSequence;
    if (
      commandSequence !== undefined &&
      (event.kind === "commandCompleted" || event.kind === "commandFailed")
    ) {
      const pending = this.pending.get(commandSequence);
      if (!pending) {
        this.emit("staleAcknowledgement", event);
        return;
      }
      this.clock.clearTimeout(pending.timer);
      this.pending.delete(commandSequence);
      if (event.commandKind !== pending.kind) {
        pending.reject(
          new Error(
            `USB acknowledgement kind mismatch: expected ${pending.kind}, received ${event.commandKind}`,
          ),
        );
      } else if (event.kind === "commandFailed") {
        pending.reject(
          new Error(
            `USB command ${pending.kind} failed: ${event.code ?? "unknown"} ${event.message ?? ""}`.trim(),
          ),
        );
      } else {
        pending.resolve(event);
      }
      return;
    }
    this.emit("event", event);
  }
}
