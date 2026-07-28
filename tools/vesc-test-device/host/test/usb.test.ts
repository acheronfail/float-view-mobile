import assert from "node:assert/strict";
import test from "node:test";
import type { LineTransport, UsbClock } from "../src/usb.js";
import { UsbControlClient } from "../src/usb.js";

class FakeTransport implements LineTransport {
  writes: string[] = [];
  handler: ((line: string) => void) | undefined;
  async write(line: string): Promise<void> { this.writes.push(line); }
  onLine(handler: (line: string) => void): () => void {
    this.handler = handler;
    return () => { this.handler = undefined; };
  }
  async close(): Promise<void> {}
  emit(value: object): void { this.handler?.(JSON.stringify(value)); }
}

class FakeClock implements UsbClock {
  next = 0;
  handlers = new Map<number, () => void>();
  setTimeout(handler: () => void): unknown {
    const id = ++this.next;
    this.handlers.set(id, handler);
    return id;
  }
  clearTimeout(handle: unknown): void { this.handlers.delete(handle as number); }
  fireAll(): void {
    const values = [...this.handlers.values()];
    this.handlers.clear();
    for (const handler of values) handler();
  }
}

function completion(sequence: number, commandSequence: number, commandKind: string) {
  return { protocolVersion: 1, sequence, kind: "commandCompleted", commandSequence, commandKind, status: "ok" };
}

test("USB client correlates acknowledgements and writes monotonically sequenced commands", async () => {
  const transport = new FakeTransport();
  const client = new UsbControlClient(transport);
  const pending = client.command("hello");
  await new Promise((resolve) => setImmediate(resolve));
  assert.match(transport.writes[0]!, /"sequence":1,"kind":"hello"/);
  transport.emit(completion(1, 1, "hello"));
  assert.equal((await pending).status, "ok");
  await client.close();
});

test("USB client rejects host command timeout", async () => {
  const transport = new FakeTransport();
  const clock = new FakeClock();
  const client = new UsbControlClient(transport, 50, clock);
  const pending = client.command("hello");
  clock.fireAll();
  await assert.rejects(pending, /timed out after 50ms/);
  await client.close();
});

test("USB client rejects pending work when the ESP resets", async () => {
  const transport = new FakeTransport();
  const client = new UsbControlClient(transport);
  const established = client.command("hello");
  transport.emit(completion(8, 1, "hello"));
  await established;
  const pending = client.command("getState");
  transport.emit({ protocolVersion: 1, sequence: 1, kind: "boot" });
  await assert.rejects(pending, /ESP reset/);
  await client.close();
});

test("USB client ignores and reports stale acknowledgements", async () => {
  const transport = new FakeTransport();
  const clock = new FakeClock();
  const client = new UsbControlClient(transport, 50, clock);
  let stale = 0;
  client.on("staleAcknowledgement", () => { stale += 1; });
  const pending = client.command("hello");
  transport.emit(completion(1, 99, "hello"));
  assert.equal(stale, 1);
  clock.fireAll();
  await assert.rejects(pending, /timed out/);
  await client.close();
});

test("USB client rejects mismatched acknowledgement kinds", async () => {
  const transport = new FakeTransport();
  const client = new UsbControlClient(transport);
  const pending = client.command("hello");
  transport.emit(completion(1, 1, "reset"));
  await assert.rejects(pending, /kind mismatch/);
  await client.close();
});
