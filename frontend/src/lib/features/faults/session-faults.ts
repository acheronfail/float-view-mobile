import type { SessionMetadata } from '$lib/backend/contract';
import type { SessionEventPage } from '$lib/backend/session-data';
import type { SessionReviewController } from '$lib/backend/session-controller';
import type { RideFault } from './faults';

interface TimeoutObservation {
  fault: RideFault;
  since: number;
  retainedStop: boolean;
}

interface TimeoutObservations {
  /** Only sampled alert state establishes activation or clearance. */
  active: boolean | null;
  lastObservedMs: number;
  observations: TimeoutObservation[];
}

interface FaultHistory {
  recovered: RideFault[];
  observed: RideFault[];
  timeout: TimeoutObservations;
}

/** Read all event pages with a fixed ceiling. Never display a truncated fault list. */
export async function loadFirmwareFaults(
  controller: Pick<SessionReviewController, 'loadEvents'>,
  metadata: SessionMetadata
): Promise<RideFault[]> {
  const history: FaultHistory = {
    recovered: [],
    observed: [],
    timeout: { active: null, lastObservedMs: 0, observations: [] },
  };
  let cursor: string | null = null;
  const cursors = new Set<string>();
  for (let pageIndex = 0; pageIndex < 200; pageIndex++) {
    const result = await controller.loadEvents({
      kind: 'events',
      range: { startElapsedMs: 0, endElapsedMs: metadata.snapshot.committedElapsedMs },
      limit: 500,
      cursor,
    });
    collectFaults(result.page, history);
    cursor = result.nextCursor;
    if (!cursor) return reconcileFaults(history);
    if (cursors.has(cursor)) throw new Error('Fault history could not be loaded completely.');
    cursors.add(cursor);
  }
  throw new Error('Fault history is too large to display.');
}

function collectFaults(page: SessionEventPage, history: FaultHistory): void {
  for (let i = 0; i < page.sequence.length; i++) {
    collectFault(page, i, history);
    if (history.recovered.length + history.observed.length + history.timeout.observations.length > 10_000)
      throw new Error('Fault history is too large to display.');
  }
}

function collectFault(page: SessionEventPage, i: number, history: FaultHistory): void {
  const { recovered, observed, timeout } = history;
  if (page.kinds[i] === 'imuTimeoutHistory') {
    const details = page.timeouts?.[i];
    if (!details) throw new Error('Timeout history is incomplete.');
    recovered.push({
      sequence: page.sequence[i]!,
      reason: details.active ? 'imuTimeout' : 'imuTimeoutCleared',
      code: null,
      elapsedMs: details.occurredElapsedMs,
      observedElapsedMs: page.elapsedMs[i]!,
    });
    return;
  }
  if (page.kinds[i] === 'imuTimeoutActive' || page.kinds[i] === 'stopCondition') {
    observeTimeout(page, i, timeout);
    return;
  }
  const reason = page.values[i]!;
  const kind = page.kinds[i];
  if (reason === 'none' || (kind !== 'firmwareFaultHistory' && kind !== 'firmwareFault')) return;
  const details = page.faults?.[i];
  const fault: RideFault = {
    sequence: page.sequence[i]!,
    reason,
    code: details?.rawCode ?? null,
    elapsedMs: kind === 'firmwareFaultHistory' ? (details?.occurredElapsedMs ?? null) : page.elapsedMs[i]!,
    observedElapsedMs: page.elapsedMs[i]!,
  };
  (kind === 'firmwareFaultHistory' ? recovered : observed).push(fault);
}

function observeTimeout(page: SessionEventPage, i: number, timeout: TimeoutObservations): void {
  if (page.kinds[i] === 'imuTimeoutActive') {
    const active = page.values[i] === 'active';
    if (active || timeout.active === true) {
      // Only the same observation proves this stop and activation coincide.
      const last = timeout.observations.at(-1);
      if (active && last?.retainedStop && last.fault.observedElapsedMs === page.elapsedMs[i])
        timeout.observations.pop();
      timeout.observations.push({
        fault: {
          sequence: page.sequence[i]!,
          reason: active ? 'imuTimeout' : 'imuTimeoutCleared',
          code: null,
          elapsedMs: page.elapsedMs[i]!,
          observedElapsedMs: page.elapsedMs[i]!,
        },
        since: timeout.lastObservedMs,
        retainedStop: false,
      });
    }
    timeout.lastObservedMs = page.elapsedMs[i]!;
    timeout.active = active;
    return;
  }
  if (page.kinds[i] === 'stopCondition' && page.values[i] === 'imuTimeout') {
    if (timeout.active !== true) {
      timeout.observations.push({
        fault: {
          sequence: page.sequence[i]!,
          reason: 'imuTimeout',
          code: null,
          elapsedMs: null,
          observedElapsedMs: page.elapsedMs[i]!,
        },
        since: 0,
        retainedStop: true,
      });
    }
    return;
  }
}

function reconcileFaults({ recovered, observed, timeout }: FaultHistory): RideFault[] {
  // Poll-time state transitions describe the same episodes recovered from history.
  const times = new Map<string, number[]>();
  for (const fault of recovered) {
    const rows = times.get(fault.reason) ?? [];
    rows.push(fault.elapsedMs ?? fault.observedElapsedMs);
    times.set(fault.reason, rows);
  }
  for (const rows of times.values()) rows.sort((a, b) => a - b);
  const faults = [
    ...recovered,
    ...unrecoveredTimeouts(timeout.observations, recovered),
    ...observed.filter((fault) => {
      const rows = times.get(fault.reason) ?? [];
      let lo = 0,
        hi = rows.length;
      while (lo < hi) {
        const mid = (lo + hi) >>> 1;
        if (rows[mid]! < fault.observedElapsedMs - 2000) lo = mid + 1;
        else hi = mid;
      }
      return lo === rows.length || rows[lo]! > fault.observedElapsedMs + 2000;
    }),
  ];
  return faults.sort(
    (a, b) => (a.elapsedMs ?? a.observedElapsedMs) - (b.elapsedMs ?? b.observedElapsedMs) || a.sequence - b.sequence
  );
}

/** Match each sampled transition once, within its state-observation window.
 * Retrieval may be arbitrarily late; it is not an occurrence-time tolerance.
 */
function unrecoveredTimeouts(observations: TimeoutObservation[], recovered: RideFault[]): RideFault[] {
  const timeouts = recovered.filter((fault) => fault.reason === 'imuTimeout' || fault.reason === 'imuTimeoutCleared');
  const unmatched: RideFault[] = [];
  let nextRecovered = 0;
  const nextTransitionTimes: (number | null | undefined)[] = new Array(timeouts.length);
  const nextTransition = new Map<string, number | null>();
  for (let i = timeouts.length - 1; i >= 0; i--) {
    const fault = timeouts[i]!;
    const opposite = fault.reason === 'imuTimeout' ? 'imuTimeoutCleared' : 'imuTimeout';
    nextTransitionTimes[i] = nextTransition.get(opposite);
    nextTransition.set(fault.reason, fault.elapsedMs);
  }
  const nextSameObservation = new Map<string, number>();
  const nextSameTimes: (number | undefined)[] = new Array(observations.length);
  for (let i = observations.length - 1; i >= 0; i--) {
    const { fault } = observations[i]!;
    nextSameTimes[i] = nextSameObservation.get(fault.reason);
    nextSameObservation.set(fault.reason, fault.observedElapsedMs);
  }
  for (let i = 0; i < observations.length; i++) {
    const { fault, since, retainedStop } = observations[i]!;
    // An undated record may match by order/observation, but not across a later
    // observation of the same transition (a separate episode).
    const nextSame = nextSameTimes[i];
    let match = -1;
    for (let j = nextRecovered; j < timeouts.length; j++) {
      const candidate = timeouts[j]!;
      if (candidate.reason !== fault.reason) continue;
      // A source episode that already ended before this state observation
      // cannot replace the observation of a later episode.
      const nextTransitionTime = nextTransitionTimes[j];
      if (!retainedStop && nextTransitionTime != null && nextTransitionTime < fault.observedElapsedMs) continue;
      if (
        candidate.elapsedMs !== null
          ? candidate.elapsedMs >= since && candidate.elapsedMs <= fault.observedElapsedMs
          : candidate.observedElapsedMs >= since && (nextSame === undefined || candidate.observedElapsedMs < nextSame)
      ) {
        match = j;
        break;
      }
    }
    if (match < 0) unmatched.push(fault);
    else nextRecovered = match + 1;
  }
  return unmatched;
}
