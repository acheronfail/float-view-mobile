import type { FootpadState, SessionMetadata } from '$lib/backend/contract';
import type { SessionReviewController } from '$lib/backend/session-controller';

export type FootpadFault = { elapsedMs: number; state: 'none' | 'left' | 'right' };
type Transition = { elapsedMs: number; sequence: number; state: FootpadState };
const MAX_EVENTS = 100_000;
const MAX_SAMPLES = 1_000_000;
const MAX_FAULTS = 2_000;

// Read exact retained speed samples: overview buckets can hide short pad dropouts.
// Keep only transitions and episode starts, never a whole ride's sample vectors.
export async function loadFootpadFaults(controller: SessionReviewController, metadata: SessionMetadata) {
  if (metadata.eventCount > MAX_EVENTS || metadata.storedSampleCount > MAX_SAMPLES)
    throw new Error('This session is too large to display footpad faults.');
  const range = { startElapsedMs: 0, endElapsedMs: metadata.summary.totalElapsedMs };
  const transitions: Transition[] = [];
  let cursor: string | null = null;
  let rows = 0;
  do {
    const result = await controller.loadEvents({ kind: 'events', range, limit: 500, cursor });
    rows += result.page.sequence.length;
    if (rows > MAX_EVENTS || (result.nextCursor && !result.page.sequence.length))
      throw new Error('Footpad transition history exceeds display limits.');
    result.page.kinds.forEach((kind, i) => {
      if (kind !== 'footpadState') return;
      const value = result.page.values[i];
      transitions.push({
        elapsedMs: result.page.elapsedMs[i]!,
        sequence: result.page.sourceSampleSequence[i]!,
        state: value === 'none' || value === 'left' || value === 'right' || value === 'both' ? value : 'unknown',
      });
    });
    cursor = result.nextCursor;
  } while (cursor);
  const detector = new FootpadFaultDetector(transitions);
  rows = 0;
  do {
    const result = await controller.loadSamples({
      kind: 'samples',
      mode: 'window',
      fields: ['speedKph'],
      range,
      limit: 2_000,
      cursor,
    });
    rows += result.sequence.length;
    if (rows > MAX_SAMPLES || (result.nextCursor && !result.sequence.length))
      throw new Error('Footpad sample history exceeds display limits.');
    result.sequence.forEach((sequence, i) =>
      detector.push(result.elapsedMs[i]!, sequence, result.values.speedKph?.[i] ?? null)
    );
    if (detector.faults.length > MAX_FAULTS) throw new Error('This session has too many footpad faults to display.');
    cursor = result.nextCursor;
  } while (cursor);
  return detector.faults;
}

export class FootpadFaultDetector {
  readonly faults: FootpadFault[] = [];
  private index = 0;
  private state: FootpadState = 'unknown';
  private previous: FootpadFault['state'] | null = null;
  private previousElapsedMs: number | null = null;
  constructor(private readonly transitions: readonly Transition[]) {}

  push(elapsedMs: number, sequence: number, speedKph: number | null) {
    while (this.index < this.transitions.length) {
      const event = this.transitions[this.index]!;
      if (event.elapsedMs > elapsedMs || (event.elapsedMs === elapsedMs && event.sequence > sequence)) break;
      if (this.state !== event.state) this.previous = null;
      this.state = event.state;
      this.index++;
    }
    const fault =
      speedKph !== null &&
      Number.isFinite(speedKph) &&
      Math.abs(speedKph) > 2 &&
      (this.state === 'none' || this.state === 'left' || this.state === 'right')
        ? this.state
        : null;
    if (
      fault &&
      (fault !== this.previous || (this.previousElapsedMs !== null && elapsedMs - this.previousElapsedMs > 10_000))
    )
      this.faults.push({ elapsedMs, state: fault });
    this.previous = fault;
    this.previousElapsedMs = elapsedMs;
  }
}

export function footpadFaultLabel(fault: FootpadFault) {
  const state =
    fault.state === 'none'
      ? 'Both footpads inactive'
      : fault.state === 'left'
        ? 'Right footpad inactive; left active'
        : 'Left footpad inactive; right active';
  return `${state} above 2 km/h at ${(fault.elapsedMs / 1000).toFixed(1)} seconds`;
}

export function footpadFaultIcon(state: FootpadFault['state']) {
  const left = state === 'left' ? '#22c55e' : '#ef4444';
  const right = state === 'right' ? '#22c55e' : '#ef4444';
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 36 28" width="36" height="28" aria-hidden="true"><rect x="1" y="1" width="34" height="26" rx="13" fill="white" stroke="#111827"/><path d="M16 4H14a10 10 0 0 0 0 20h2Z" fill="${left}" stroke="#111827"/><path d="M20 4h2a10 10 0 0 1 0 20h-2Z" fill="${right}" stroke="#111827"/></svg>`;
}
