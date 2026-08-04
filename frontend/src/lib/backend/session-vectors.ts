import type { ReferenceEvent, ReferenceSample } from './session-reference.ts';
import type { SessionDataRange, SessionGapPolicy } from './session-data.ts';
import type { SessionPublishedSnapshot } from './contract.ts';

/**
 * Small, platform-neutral historical fixtures.  Keep these values literal and
 * deeply frozen: Swift/Kotlin ports can consume the same JSON representation
 * without inheriting a reducer implementation from the frontend.
 */
export interface SessionDataVector {
  readonly name: string;
  readonly samples: readonly ReferenceSample[];
  readonly events: readonly ReferenceEvent[];
  readonly range: SessionDataRange;
  readonly snapshot: SessionPublishedSnapshot;
  readonly fields: readonly string[];
  readonly policies: readonly SessionGapPolicy[];
  readonly buckets: readonly number[];
  readonly expectedGenerations: Readonly<Record<string, readonly number[]>>;
  readonly expectedSegments: Readonly<Record<string, readonly (number | null)[]>>;
  readonly pruneResolutionSeconds: readonly number[];
}

const freeze = <T>(value: T): T => {
  if (value && typeof value === 'object' && !Object.isFrozen(value)) {
    Object.values(value as Record<string, unknown>).forEach((child) => freeze(child));
    Object.freeze(value);
  }
  return value;
};

export const SESSION_DATA_VECTORS: readonly SessionDataVector[] = freeze([
  {
    name: 'edges-ties-gaps-and-late-fields',
    samples: [
      { sequence: 10, elapsedMs: 0, rideState: 'idle', values: { speedKph: 0, packVoltageV: 60, extensionLate: null } },
      { sequence: 11, elapsedMs: 0, rideState: 'idle', values: { speedKph: 1, packVoltageV: 59, extensionLate: null } },
      {
        sequence: 12,
        elapsedMs: 999,
        rideState: 'riding',
        values: { speedKph: 10, packVoltageV: 58, extensionLate: 0.5 },
      },
      {
        sequence: 13,
        elapsedMs: 1_000,
        rideState: 'riding',
        values: { speedKph: 20, packVoltageV: 57, extensionLate: 0.6 },
      },
      {
        sequence: 14,
        elapsedMs: 1_001,
        rideState: 'riding',
        values: { speedKph: 5, packVoltageV: 56, extensionLate: 0.7 },
      },
      {
        sequence: 15,
        elapsedMs: 3_500,
        rideState: 'riding',
        values: { speedKph: null, packVoltageV: 55, extensionLate: 0.8 },
      },
      {
        sequence: 16,
        elapsedMs: 3_501,
        rideState: 'riding',
        values: { speedKph: 7, packVoltageV: null, extensionLate: 0.9 },
      },
      {
        sequence: 17,
        elapsedMs: 10_000,
        rideState: 'idle',
        values: { speedKph: 2, packVoltageV: 54, extensionLate: 1 },
      },
      {
        sequence: 18,
        elapsedMs: 10_000,
        rideState: 'idle',
        values: { speedKph: 3, packVoltageV: 53, extensionLate: 1.1 },
      },
    ],
    events: [
      { sequence: 2, sourceSampleSequence: 12, elapsedMs: 999, kind: 'packageState', value: 'running' },
      { sequence: 3, sourceSampleSequence: 15, elapsedMs: 3_500, kind: 'footpadState', value: 'none' },
      { sequence: 4, sourceSampleSequence: 17, elapsedMs: 10_000, kind: 'packageState', value: 'stopped' },
    ],
    range: { startElapsedMs: 0, endElapsedMs: 10_000 },
    snapshot: {
      historyRevision: 7,
      committedSampleSequence: 18,
      committedEventSequence: 4,
      committedElapsedMs: 10_000,
      committedLocationSequence: 0,
      committedLocationElapsedMs: 0,
    },
    fields: ['speedKph', 'packVoltageV', 'extensionLate'],
    policies: [{ thresholdSeconds: 2 }, { thresholdSeconds: null }],
    buckets: [1, 2, 3, 4],
    expectedGenerations: {
      speedKph: [0, 0, 0, 2, 3, 3],
      packVoltageV: [0, 0, 1, 3, 3],
      extensionLate: [2, 2, 3, 3, 4, 4],
    },
    expectedSegments: {
      speedKph: [0, 20, 5, 7, 2, 3],
      packVoltageV: [60, 56, 55, 54, 53],
      extensionLate: [0.5, 0.7, 0.8, 0.9, 1, 1.1],
    },
    pruneResolutionSeconds: [1, 5],
  },
  {
    name: 'dense-pages-and-cursor-replay',
    samples: Array.from({ length: 12 }, (_, index) => ({
      sequence: index + 1,
      elapsedMs: index * 100,
      rideState: index < 6 ? 'riding' : 'idle',
      values: { speedKph: index, extensionLate: index >= 6 ? index / 10 : null },
    })),
    events: Array.from({ length: 6 }, (_, index) => ({
      sequence: index + 1,
      sourceSampleSequence: index * 2 + 1,
      elapsedMs: index * 200,
      kind: index % 2 ? 'charging' : 'packageState',
      value: index % 2 ? 'active' : 'running',
    })),
    range: { startElapsedMs: 0, endElapsedMs: 1_100 },
    snapshot: {
      historyRevision: 3,
      committedSampleSequence: 12,
      committedEventSequence: 6,
      committedElapsedMs: 1_100,
      committedLocationSequence: 0,
      committedLocationElapsedMs: 0,
    },
    fields: ['speedKph', 'extensionLate'],
    policies: [{ thresholdSeconds: 0.1 }],
    buckets: [4],
    expectedGenerations: { speedKph: [0, 0, 0, 0, 0, 0] },
    expectedSegments: { speedKph: [0, 1, 2, 3, 4, 5] },
    nearestTargetElapsedMs: 550,
    pruneResolutionSeconds: [1],
  },
] as const);

export const SESSION_DATA_VECTOR_JSON = JSON.stringify(SESSION_DATA_VECTORS);
