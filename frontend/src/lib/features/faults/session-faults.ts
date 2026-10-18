import type { SessionMetadata } from '$lib/backend/contract';
import type { SessionReviewController } from '$lib/backend/session-controller';
import type { RideFault } from './faults';

/** Read all event pages with a fixed ceiling. Never display a truncated fault list. */
export async function loadFirmwareFaults(
  controller: Pick<SessionReviewController, 'loadEvents'>,
  metadata: SessionMetadata
): Promise<RideFault[]> {
  const recovered: RideFault[] = [],
    observed: RideFault[] = [];
  let cursor: string | null = null;
  const cursors = new Set<string>();
  for (let pageIndex = 0; pageIndex < 200; pageIndex++) {
    const result = await controller.loadEvents({
      kind: 'events',
      range: { startElapsedMs: 0, endElapsedMs: metadata.snapshot.committedElapsedMs },
      limit: 500,
      cursor,
    });
    const page = result.page;
    for (let i = 0; i < page.sequence.length; i++) {
      const reason = page.values[i]!;
      const kind = page.kinds[i];
      if (reason === 'none' || (kind !== 'firmwareFaultHistory' && kind !== 'firmwareFault')) continue;
      const details = page.faults?.[i];
      const fault: RideFault = {
        sequence: page.sequence[i]!,
        reason,
        code: details?.rawCode ?? null,
        elapsedMs: kind === 'firmwareFaultHistory' ? (details?.occurredElapsedMs ?? null) : page.elapsedMs[i]!,
        observedElapsedMs: page.elapsedMs[i]!,
      };
      (kind === 'firmwareFaultHistory' ? recovered : observed).push(fault);
      if (recovered.length + observed.length > 10_000) throw new Error('Fault history is too large to display.');
    }
    cursor = result.nextCursor;
    if (!cursor) {
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
    if (cursors.has(cursor)) throw new Error('Fault history could not be loaded completely.');
    cursors.add(cursor);
  }
  throw new Error('Fault history is too large to display.');
}
