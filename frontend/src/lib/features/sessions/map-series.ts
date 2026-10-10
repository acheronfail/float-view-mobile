import type { SessionReviewController } from '$lib/backend/session-controller';
import { sessionDataDecodedBytes, type SessionSeriesResult, type SessionSeriesColumn } from '$lib/backend/session-data';
import type { Viewport } from './viewport';
import { intervalBuckets, MAX_MAP_INTERVALS } from './map-filter';

const abort = () => new DOMException('Superseded map selection', 'AbortError');
const MAX_BYTES = 16 * 1024 * 1024;

/** One bounded scheduler for all visible field groups of a map selection. */
export class MapSeriesLoader {
  private generation = 0;
  private running = 0;
  private queue: { run: () => void; reject: (reason: unknown) => void }[] = [];
  private bytes = new Map<string, number>();
  private pendingBytes = 0;
  constructor(private controller: SessionReviewController) {}

  cancel() {
    this.generation++;
    this.controller.cancelPending();
    for (const item of this.queue.splice(0)) item.reject(abort());
    this.bytes.clear();
    this.pendingBytes = 0;
  }

  private schedule<T>(generation: number, work: () => Promise<T>): Promise<T> {
    return new Promise((resolve, reject) => {
      const run = () => {
        if (generation !== this.generation) {
          reject(abort());
          return;
        }
        this.running++;
        void work()
          .then((value) => {
            if (generation !== this.generation) throw abort();
            resolve(value);
          })
          .catch(reject)
          .finally(() => {
            this.running--;
            this.queue.shift()?.run();
          });
      };
      if (this.running < 2) run();
      else this.queue.push({ run, reject });
    });
  }

  async load(
    fields: string[],
    intervals: readonly Viewport[],
    width: number,
    threshold: number | null
  ): Promise<SessionSeriesResult> {
    if (!intervals.length || intervals.length > MAX_MAP_INTERVALS)
      throw new Error('Invalid map interval count. Zoom out or disable map filtering.');
    const generation = this.generation;
    const buckets = intervalBuckets(intervals, width);
    let reserved = 0;
    const key = fields.join('\u0000');
    try {
      // Two workers per group bound the queue to active field groups, not interval count.
      let next = 0;
      const results: SessionSeriesResult[] = [];
      const worker = async () => {
        while (next < intervals.length) {
          const index = next++,
            range = intervals[index]!;
          results[index] = await this.schedule(generation, async () => {
            const result = await this.controller.loadSeries(
              fields,
              range.startElapsedMs,
              range.endElapsedMs,
              buckets[index]!,
              threshold,
              `map:${index}`
            );
            if (generation !== this.generation) throw abort();
            const size = sessionDataDecodedBytes(result) ?? JSON.stringify(result).length * 2;
            if (this.pendingBytes + [...this.bytes.values()].reduce((a, b) => a + b, 0) + size > MAX_BYTES)
              throw new Error('Filtered chart data exceeds the review limit. Zoom in or disable map filtering.');
            this.pendingBytes += size;
            reserved += size;
            return result;
          });
        }
      };
      const workers = await Promise.allSettled([worker(), worker()]);
      const failed = workers.find((r) => r.status === 'rejected');
      if (failed?.status === 'rejected') throw failed.reason;
      if (generation !== this.generation) throw abort();
      const first = results[0]!;
      const columns: Record<string, SessionSeriesColumn> = {};
      for (const field of fields) {
        const column: SessionSeriesColumn = { elapsedMs: [], sequence: [], values: [], generations: [] };
        let generationOffset = 0;
        results.forEach((result, index) => {
          const source = result.columns[field];
          if (!source) return;
          const range = intervals[index]!;
          let maxGeneration = generationOffset;
          source.elapsedMs.forEach((elapsed, i) => {
            if (elapsed < range.startElapsedMs || elapsed > range.endElapsedMs) return;
            column.elapsedMs.push(elapsed);
            column.sequence.push(source.sequence[i]!);
            column.values.push(source.values[i]!);
            const value = generationOffset + source.generations[i]!;
            column.generations.push(value);
            maxGeneration = Math.max(maxGeneration, value);
          });
          generationOffset = maxGeneration + 1;
        });
        columns[field] = column;
      }
      this.bytes.set(key, reserved);
      return {
        ...first,
        range: { startElapsedMs: intervals[0]!.startElapsedMs, endElapsedMs: intervals.at(-1)!.endElapsedMs },
        columns,
      };
    } finally {
      if (generation === this.generation) this.pendingBytes -= reserved;
    }
  }
}
