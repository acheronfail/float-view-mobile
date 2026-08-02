import type { ReferenceEvent, ReferenceSample } from './session-reference.ts';

export interface PruneVector {
  readonly name: string;
  readonly samples: readonly ReferenceSample[];
  readonly events: readonly ReferenceEvent[];
  readonly fields: readonly string[];
  readonly protectedSequences: readonly number[];
  readonly recordedSampleCount: number;
  readonly beforeRevision: number;
  readonly afterRevision: number;
  readonly expectedRetainedByResolution: Readonly<Record<number, readonly number[]>>;
}

const freeze = <T>(value: T): T => {
  if (value && typeof value === 'object' && !Object.isFrozen(value)) {
    Object.values(value as Record<string, unknown>).forEach((child) => freeze(child));
    Object.freeze(value);
  }
  return value;
};

const samples: ReferenceSample[] = Array.from({ length: 24 }, (_, sequence) => ({
  sequence,
  elapsedMs: sequence * 500,
  rideState: sequence < 12 ? 'riding' : 'idle',
  values: {
    speedKph: sequence === 6 ? null : (sequence * 3) % 19,
    packVoltageV: 60 - sequence * 0.2,
    extensionLate: sequence >= 12 ? sequence / 10 : null,
  },
}));

export const PRUNE_VECTORS: readonly PruneVector[] = freeze([
  {
    name: 'protected-boundaries-and-greedy-coverage',
    samples,
    events: [
      { sequence: 1, sourceSampleSequence: 5, elapsedMs: 2_500, kind: 'packageState', value: 'running' },
      { sequence: 2, sourceSampleSequence: 12, elapsedMs: 6_000, kind: 'packageState', value: 'stopped' },
    ],
    fields: ['speedKph', 'packVoltageV', 'extensionLate'],
    protectedSequences: [5, 12, 13],
    recordedSampleCount: samples.length,
    beforeRevision: 4,
    afterRevision: 5,
    expectedRetainedByResolution: {
      1: Array.from({ length: 24 }, (_, sequence) => sequence),
      5: [0, 5, 9, 10, 12, 13, 19, 20, 23],
    },
  },
] as const);

export const PRUNE_VECTOR_JSON = JSON.stringify(PRUNE_VECTORS);
