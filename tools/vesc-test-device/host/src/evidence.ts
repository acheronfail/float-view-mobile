import { mkdir, writeFile } from "node:fs/promises";
import { randomBytes } from "node:crypto";
import { join } from "node:path";
import type { TranscriptEntry, VerificationManifest } from "./types.js";

export function physicalRunLabel(...parts: string[]): string {
  const prefix = parts.join("-");
  if (!/^[a-z0-9][a-z0-9-]+$/.test(prefix)) {
    throw new Error("physical-run evidence label is unsafe");
  }
  return `${prefix}-${Date.now().toString(36)}-${randomBytes(4).toString("hex")}`;
}

export async function writeEvidence(
  directory: string,
  manifest: VerificationManifest,
  transcript: readonly TranscriptEntry[],
  label?: string,
): Promise<{ manifestPath: string; transcriptPath: string }> {
  if (!/^[a-z0-9][a-z0-9-]+$/.test(manifest.scenarioId)) {
    throw new Error("scenario ID is unsafe for an evidence filename");
  }
  if (label !== undefined && !/^[a-z0-9][a-z0-9-]+$/.test(label)) {
    throw new Error("evidence label is unsafe for an evidence filename");
  }
  await mkdir(directory, { recursive: true });
  const prefix = [
    manifest.scenarioId,
    label,
    manifest.passed ? "passed" : "failed",
  ].filter(Boolean).join("-");
  const manifestPath = join(directory, `${prefix}-manifest.json`);
  const transcriptPath = join(directory, `${prefix}-transcript.json`);
  await Promise.all([
    writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, { flag: "wx" }),
    writeFile(
      transcriptPath,
      `${JSON.stringify({ schemaVersion: 1, scenarioId: manifest.scenarioId, entries: transcript }, null, 2)}\n`,
      { flag: "wx" },
    ),
  ]);
  return { manifestPath, transcriptPath };
}
