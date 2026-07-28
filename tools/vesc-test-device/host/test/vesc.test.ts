import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { resolve } from "node:path";
import { fromHex, toHex } from "../src/bytes.js";
import { repositoryRoot } from "../src/scenario.js";
import { crc16, decodeFrame, encodeFrame, VescStreamAssembler } from "../src/vesc.js";

test("CRC and frame codec reproduce the committed firmware fixture byte-for-byte", async () => {
  const fixture = JSON.parse(
    await readFile(resolve(repositoryRoot, "fixtures/vesc/firmware-6.06.json"), "utf8"),
  ) as { frameHex: string };
  const frame = fromHex(fixture.frameHex);
  const payload = decodeFrame(frame);
  assert.equal(crc16(payload), 0x2ad8);
  assert.equal(toHex(encodeFrame(payload)), fixture.frameHex);
});

test("stream assembler retains partial frames and emits only after completion", () => {
  const frame = encodeFrame(Uint8Array.of(0));
  const stream = new VescStreamAssembler();
  assert.deepEqual(stream.append(frame.slice(0, 3)), []);
  const results = stream.append(frame.slice(3));
  assert.equal(results.length, 1);
  assert.equal(results[0]?.kind, "frame");
  if (results[0]?.kind === "frame") assert.deepEqual([...results[0].payload], [0]);
});

test("stream assembler skips noise and emits combined frames in order", () => {
  const first = encodeFrame(Uint8Array.of(0));
  const second = encodeFrame(Uint8Array.of(62));
  const stream = new VescStreamAssembler();
  const combined = Uint8Array.of(0xff, 1, ...first, ...second);
  const frames = stream.append(combined).filter((result) => result.kind === "frame");
  assert.deepEqual(frames.map((result) => [...result.payload]), [[0], [62]]);
});

test("stream assembler reports CRC corruption and recovers for the next frame", () => {
  const corrupt = encodeFrame(Uint8Array.of(0));
  corrupt[corrupt.length - 2] = corrupt[corrupt.length - 2]! ^ 1;
  const valid = encodeFrame(Uint8Array.of(62));
  const results = new VescStreamAssembler().append(Uint8Array.of(...corrupt, ...valid));
  assert.deepEqual(results.map((result) => result.kind), ["fault", "frame"]);
  assert.equal(results[0]?.kind === "fault" && results[0].code, "invalid_crc");
});

test("long VESC frames use the three-byte header", () => {
  const payload = new Uint8Array(256);
  payload[0] = 4;
  const frame = encodeFrame(payload);
  assert.deepEqual([...frame.slice(0, 3)], [3, 1, 0]);
  assert.deepEqual(decodeFrame(frame), payload);
});
