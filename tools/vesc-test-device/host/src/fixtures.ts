import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fromHex } from "./bytes.js";
import { repositoryRoot } from "./scenario.js";

interface FixtureCase {
  name: string;
  frameHex: string;
  chunkSizes?: number[];
}

export async function loadFixture(reference: string, caseName?: string): Promise<{ frame: Uint8Array; chunkSizes?: number[] }> {
  const path = resolve(repositoryRoot, reference);
  if (!path.startsWith(`${repositoryRoot}/`)) throw new Error(`fixture escapes repository: ${reference}`);
  const fixture = JSON.parse(await readFile(path, "utf8")) as {
    schemaVersion?: number;
    frameHex?: string;
    chunkSizes?: number[];
    cases?: FixtureCase[];
  };
  if (fixture.schemaVersion !== 1) throw new Error(`unsupported fixture schema in ${reference}`);
  if (caseName !== undefined) {
    const selected = fixture.cases?.find((entry) => entry.name === caseName);
    if (!selected) throw new Error(`fixture case ${caseName} not found in ${reference}`);
    return optionalChunks(fromHex(selected.frameHex), selected.chunkSizes);
  }
  if (!fixture.frameHex) throw new Error(`fixture ${reference} requires a case`);
  return optionalChunks(fromHex(fixture.frameHex), fixture.chunkSizes);
}

function optionalChunks(frame: Uint8Array, chunkSizes: number[] | undefined): { frame: Uint8Array; chunkSizes?: number[] } {
  return chunkSizes === undefined ? { frame } : { frame, chunkSizes };
}
