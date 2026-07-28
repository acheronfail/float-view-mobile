import { execFile } from "node:child_process";
import { existsSync } from "node:fs";
import { promisify } from "node:util";
import { SerialPort } from "serialport";
import type { UsbControlClient } from "./usb.js";

const execFileAsync = promisify(execFile);

interface PortInfo {
  path: string;
  manufacturer?: string | undefined;
  vendorId?: string | undefined;
}

export function selectEspPort(
  ports: readonly PortInfo[],
  override = process.env.FLOAT_VIEW_ESP_PORT,
): string {
  if (override) return override;
  const matches = ports.filter((port) => {
    const manufacturer = port.manufacturer?.toLowerCase() ?? "";
    const vendorId = port.vendorId?.toLowerCase() ?? "";
    return manufacturer.includes("espressif") || vendorId === "303a";
  });
  if (matches.length !== 1) {
    throw new Error(
      `expected exactly one Espressif control port, found ${matches.length}; set FLOAT_VIEW_ESP_PORT`,
    );
  }
  return matches[0]!.path;
}

export function selectAndroidSerial(
  adbOutput: string,
  override = process.env.ANDROID_SERIAL,
): string {
  if (override) return override;
  const devices = adbOutput
    .split(/\r?\n/)
    .map((line) => line.trim().split(/\s+/))
    .filter((fields) => fields.length >= 2 && fields[1] === "device")
    .map((fields) => fields[0]!);
  if (devices.length !== 1) {
    throw new Error(
      `expected exactly one authorized Android destination, found ${devices.length}; set ANDROID_SERIAL`,
    );
  }
  return devices[0]!;
}

export async function resolveEspPort(): Promise<string> {
  const path = selectEspPort(await SerialPort.list());
  const calloutPath = path.replace(/^\/dev\/tty\./, "/dev/cu.");
  return calloutPath !== path && existsSync(calloutPath) ? calloutPath : path;
}

export async function resolveAndroidSerial(): Promise<string> {
  const result = await execFileAsync("adb", ["devices"]);
  return selectAndroidSerial(result.stdout);
}

export async function establishControlSession(
  control: UsbControlClient,
  maximumAttempts = 2,
): Promise<void> {
  let lastError: unknown;
  for (let attempt = 1; attempt <= maximumAttempts; attempt += 1) {
    try {
      await control.command("hello");
      return;
    } catch (error) {
      lastError = error;
    }
  }
  throw lastError;
}

/**
 * Allocates a large monotonically increasing range for short-lived physical
 * host processes. One range advances by 100 per second, comfortably above the
 * bounded keepalive/control command rate, without weakening the ESP's stale
 * command rejection.
 */
export function physicalHostSequenceSeed(nowMs = Date.now()): number {
  const epoch2026 = Date.UTC(2026, 0, 1);
  return Math.max(0, Math.floor((nowMs - epoch2026) / 10));
}
