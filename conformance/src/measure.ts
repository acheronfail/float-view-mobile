import { gzipSync } from 'node:zlib';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { reduceSeries, type ReferenceSample } from '../../frontend/src/lib/backend/session-reference.ts';

const SAMPLE_COUNT = 150_000;
const RUNS = 3;
const output =
  process.argv[2] ??
  resolve(import.meta.dirname, '../../openspec/changes/performance-session-visualisations/evidence/software-measurements.json');

const source: ReferenceSample[] = Array.from({ length: SAMPLE_COUNT }, (_, index) => ({
  sequence: index + 1,
  elapsedMs: index * 1_000,
  rideState: index % 20_000 < 18_000 ? 'riding' : 'idle',
  values: {
    speedKph: ((index * 17) % 420) / 10,
    packVoltageV: 58 - (index % 2_000) / 200,
    motorCurrentA: (((index * 7) % 1_800) - 900) / 10,
    controllerTempC: 25 + (index % 600) / 10,
  },
}));
const range = { startElapsedMs: 0, endElapsedMs: (SAMPLE_COUNT - 1) * 1_000 };
const fields = ['speedKph', 'packVoltageV', 'motorCurrentA', 'controllerTempC'] as const;
const timings: Array<Record<string, number>> = [];
let responseBytes = 0;
let gzipBytes = 0;
let observationCount = 0;
let pathCount = 0;

for (let run = 0; run < RUNS; run += 1) {
  const started = performance.now();
  const reduced = reduceSeries(
    source,
    fields,
    range,
    1_024,
    { thresholdSeconds: null },
    {
      expectedPersistenceMs: 1_000,
    }
  );
  const reducedMs = performance.now() - started;
  const payload = JSON.stringify(reduced);
  const encoded = new TextEncoder().encode(payload);
  const gzipStarted = performance.now();
  const compressed = gzipSync(encoded, { level: 1 });
  const gzipMs = performance.now() - gzipStarted;
  const renderStarted = performance.now();
  let paths = 0;
  for (const field of fields) {
    let inPath = false;
    const column = reduced[field]!;
    column.values.forEach((value, index) => {
      const generationBreak = index > 0 && column.generations[index] !== column.generations[index - 1];
      if (value === null || generationBreak) {
        inPath = false;
      } else if (!inPath) {
        paths += 1;
        inPath = true;
      }
    });
  }
  const renderMs = performance.now() - renderStarted;
  responseBytes = encoded.byteLength;
  gzipBytes = compressed.byteLength;
  observationCount = Object.values(reduced).reduce((sum, column) => sum + column.values.length, 0);
  pathCount = paths;
  timings.push({ reducedMs, gzipMs, renderMs });
}

const median = (key: string) => {
  const values = timings.map((item) => item[key] ?? 0).sort((left, right) => left - right);
  return values[Math.floor(values.length / 2)] ?? 0;
};
const report = {
  generatedAt: new Date().toISOString(),
  runtime: process.version,
  fixture: { samples: SAMPLE_COUNT, fields: fields.length, buckets: 1_024 },
  runs: timings,
  medianMs: { reduced: median('reducedMs'), gzip: median('gzipMs'), render: median('renderMs') },
  response: { decodedBytes: responseBytes, gzipBytes, observations: observationCount, svgPathSegments: pathCount },
  notes: [
    'Deterministic software probe; it does not substitute for the recorded iPhone 12/Galaxy S9 empty-cache checkpoint.',
    'Native storage query-plan and physical stage timings remain separate diagnostics.',
  ],
};
mkdirSync(dirname(output), { recursive: true });
writeFileSync(output, `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify(report, null, 2));
