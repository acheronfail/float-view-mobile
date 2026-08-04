import type { LocationAuthorizationStatus } from '../../frontend/src/lib/backend/contract.ts';

export interface LocationFixtureObservation {
  elapsedMs: number;
  latitude: number;
  longitude: number;
  accuracyM: number | null;
}

export interface LocationFixturePermissionTransition {
  observationIndex: number;
  status: LocationAuthorizationStatus;
  backgroundCapture: 'available' | 'foregroundOnly' | 'unavailable';
}

export interface LocationFixtureOptions {
  cadenceMs: number;
  observations: readonly LocationFixtureObservation[];
  initialStatus?: LocationAuthorizationStatus;
  initialBackgroundCapture?: LocationFixturePermissionTransition['backgroundCapture'];
  permissionTransitions?: readonly LocationFixturePermissionTransition[];
  interruptAtObservation?: number;
}

export interface LocationFixtureSnapshot {
  started: boolean;
  startCount: number;
  stopCount: number;
  emittedCount: number;
  attemptedCount: number;
  updateElapsedMs: number[];
  interruptions: number[];
  permissionTransitions: LocationFixturePermissionTransition[];
}

export type LocationFixtureListener = (observation: LocationFixtureObservation) => void;

/** A deterministic provider model used by conformance and native adapter tests. */
export class DeterministicLocationFixture {
  private readonly options: LocationFixtureOptions;
  private readonly transitions: readonly LocationFixturePermissionTransition[];
  private listener: LocationFixtureListener | null = null;
  private index = 0;
  private started = false;
  private startCount = 0;
  private stopCount = 0;
  private emittedCount = 0;
  private attemptedCount = 0;
  private readonly updateElapsedMs: number[] = [];
  private readonly interruptions: number[] = [];
  private readonly appliedTransitions: LocationFixturePermissionTransition[] = [];

  constructor(options: LocationFixtureOptions) {
    if (!Number.isSafeInteger(options.cadenceMs) || options.cadenceMs < 1) {
      throw new Error('Location fixture cadence must be a positive integer');
    }
    this.options = options;
    this.transitions = [...(options.permissionTransitions ?? [])].sort(
      (left, right) => left.observationIndex - right.observationIndex
    );
  }

  get cadenceMs(): number {
    return this.options.cadenceMs;
  }

  get status(): LocationAuthorizationStatus {
    return this.transitions.at(-1)?.status ?? this.options.initialStatus ?? 'authorized';
  }

  start(listener: LocationFixtureListener): boolean {
    if (this.started) return false;
    this.started = true;
    this.startCount += 1;
    this.listener = listener;
    return true;
  }

  stop(): boolean {
    if (!this.started) return false;
    this.started = false;
    this.stopCount += 1;
    this.listener = null;
    return true;
  }

  emitNext(): LocationFixtureObservation | null {
    const observation = this.options.observations[this.index];
    if (!observation) return null;
    this.applyTransitions(this.index);
    this.index += 1;
    this.attemptedCount += 1;
    if (this.options.interruptAtObservation === this.index - 1) {
      this.interruptions.push(observation.elapsedMs);
      return null;
    }
    if (!this.started || !this.canCapture()) return null;
    this.emittedCount += 1;
    this.updateElapsedMs.push(observation.elapsedMs);
    this.listener?.(observation);
    return observation;
  }

  emitAll(): LocationFixtureObservation[] {
    const emitted: LocationFixtureObservation[] = [];
    while (this.index < this.options.observations.length) {
      const observation = this.emitNext();
      if (observation) emitted.push(observation);
    }
    return emitted;
  }

  snapshot(): LocationFixtureSnapshot {
    return {
      started: this.started,
      startCount: this.startCount,
      stopCount: this.stopCount,
      emittedCount: this.emittedCount,
      attemptedCount: this.attemptedCount,
      updateElapsedMs: [...this.updateElapsedMs],
      interruptions: [...this.interruptions],
      permissionTransitions: [...this.appliedTransitions],
    };
  }

  private applyTransitions(observationIndex: number) {
    for (const transition of this.transitions) {
      if (transition.observationIndex !== observationIndex || this.appliedTransitions.includes(transition)) continue;
      this.appliedTransitions.push(transition);
    }
  }

  private canCapture(): boolean {
    const transition = this.appliedTransitions.at(-1);
    const status = transition?.status ?? this.options.initialStatus ?? 'authorized';
    return status === 'authorized' || status === 'foregroundOnly';
  }
}

export function defaultLocationFixture(overrides: Partial<LocationFixtureOptions> = {}): DeterministicLocationFixture {
  return new DeterministicLocationFixture({
    cadenceMs: 1_000,
    observations: [
      { elapsedMs: 1_000, latitude: -33.8688, longitude: 151.2093, accuracyM: 4.5 },
      { elapsedMs: 2_000, latitude: -33.869, longitude: 151.2098, accuracyM: null },
      { elapsedMs: 20_000, latitude: -33.87, longitude: 151.2105, accuracyM: 12 },
    ],
    ...overrides,
  });
}
