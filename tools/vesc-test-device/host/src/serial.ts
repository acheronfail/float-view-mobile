import { SerialPort } from "serialport";
import type { LineTransport } from "./usb.js";

export class SerialLineTransport implements LineTransport {
  private buffer = "";
  private readonly handlers = new Set<(line: string) => void>();
  private readonly port: SerialPort;
  private readonly opened: Promise<void>;

  constructor(path: string) {
    this.port = new SerialPort({ path, baudRate: 115_200, autoOpen: false, lock: true });
    this.opened = new Promise((resolve, reject) => {
      this.port.open((error) => error ? reject(error) : resolve());
    });
    this.port.setEncoding("utf8");
    this.port.on("data", (chunk: string) => {
      this.buffer += chunk;
      while (this.buffer.includes("\n")) {
        const index = this.buffer.indexOf("\n");
        const line = this.buffer.slice(0, index).trim();
        this.buffer = this.buffer.slice(index + 1);
        if (line.startsWith("{")) for (const handler of this.handlers) handler(line);
      }
    });
  }

  async write(line: string): Promise<void> {
    await this.opened;
    return new Promise((resolve, reject) => {
      this.port.write(line, "utf8", (error) => {
        if (error) reject(error);
        else this.port.drain((drainError) => drainError ? reject(drainError) : resolve());
      });
    });
  }

  onLine(handler: (line: string) => void): () => void {
    this.handlers.add(handler);
    return () => this.handlers.delete(handler);
  }

  async close(): Promise<void> {
    try {
      await this.opened;
    } catch {
      return;
    }
    return new Promise((resolve, reject) => {
      if (!this.port.isOpen) {
        resolve();
        return;
      }
      this.port.close((error) => error ? reject(error) : resolve());
    });
  }
}
