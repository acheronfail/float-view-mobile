import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import type { TranscriptEntry, VerificationManifest } from "./types.js";

export async function writeEvidence(
  directory: string,
  manifest: VerificationManifest,
  transcript: readonly TranscriptEntry[],
): Promise<{ manifestPath: string; transcriptPath: string }> {
  if (!/^[a-z0-9][a-z0-9-]+$/.test(manifest.scenarioId)) {
    throw new Error("scenario ID is unsafe for an evidence filename");
  }
  await mkdir(directory, { recursive: true });
  const prefix = `${manifest.scenarioId}-${manifest.passed ? "passed" : "failed"}`;
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
