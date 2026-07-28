import assert from "node:assert/strict";
import test from "node:test";
import {
  establishControlSession,
  physicalHostSequenceSeed,
  selectAndroidSerial,
  selectEspPort,
} from "../src/lab.js";

test("selectEspPort chooses the single Espressif control port", () => {
  assert.equal(
    selectEspPort(
      [
        { path: "/dev/tty.phone", manufacturer: "SAMSUNG" },
        { path: "/dev/tty.esp", manufacturer: "Espressif" },
      ],
      "",
    ),
    "/dev/tty.esp",
  );
});

test("selectEspPort requires an explicit override when discovery is ambiguous", () => {
  assert.throws(
    () =>
      selectEspPort(
        [
          { path: "/dev/tty.esp-a", vendorId: "303A" },
          { path: "/dev/tty.esp-b", manufacturer: "Espressif" },
        ],
        "",
      ),
    /exactly one Espressif/,
  );
  assert.equal(selectEspPort([], "/dev/tty/fixture"), "/dev/tty/fixture");
});

test("selectAndroidSerial chooses one authorized device and ignores offline devices", () => {
  assert.equal(
    selectAndroidSerial(
      "List of devices attached\nandroid-1\tdevice\nandroid-2\toffline\n",
      "",
    ),
    "android-1",
  );
});

test("selectAndroidSerial requires an override for zero or multiple devices", () => {
  assert.throws(
    () => selectAndroidSerial("List of devices attached\n", ""),
    /exactly one/,
  );
  assert.throws(
    () =>
      selectAndroidSerial(
        "List of devices attached\na\tdevice\nb\tdevice\n",
        "",
      ),
    /exactly one/,
  );
  assert.equal(selectAndroidSerial("", "chosen"), "chosen");
});

test("establishControlSession retries one lost startup hello", async () => {
  let attempts = 0;
  const control = {
    command: async (kind: string) => {
      assert.equal(kind, "hello");
      attempts += 1;
      if (attempts === 1) throw new Error("startup command lost");
      return {};
    },
  };
  await establishControlSession(
    control as Parameters<typeof establishControlSession>[0],
  );
  assert.equal(attempts, 2);
});

test("physical host sequence ranges advance faster than bounded control traffic", () => {
  const start = Date.UTC(2026, 6, 28);
  assert.equal(
    physicalHostSequenceSeed(start + 1_000) - physicalHostSequenceSeed(start),
    100,
  );
});
