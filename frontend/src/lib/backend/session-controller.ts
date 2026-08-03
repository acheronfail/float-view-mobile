import type { SessionMetadata } from './contract.ts';
import type {
  SessionDataRequest,
  SessionDataResult,
  SessionEventsRequest,
  SessionSamplesRequest,
  SessionSeriesResult,
  SessionSamplesResult,
  SessionEventsResult,
} from './session-data.ts';
import { sessionDataDecodedBytes } from './session-data.ts';
import { SessionDataClient } from './session-data-client';
import { reportFrontendPerformance } from './performance-log';

const MAX_CACHE_ENTRIES = 32;
const MAX_CACHE_BYTES = 8 * 1024 * 1024;

export class SessionReviewController {
  readonly metadata: SessionMetadata;
  private readonly client: SessionDataClient;
  private readonly cache = new Map<string, { value: SessionDataResult; bytes: number }>();
  private readonly generations = new Map<string, number>();
  private disposed = false;
  private readonly activeAbort = new Map<string, AbortController>();
  private cacheBytes = 0;

  constructor(metadata: SessionMetadata, client: SessionDataClient) {
    this.metadata = metadata;
    this.client = client;
  }

  get cacheLimits() {
    return SESSION_REVIEW_CACHE_LIMITS;
  }

  get cacheStats() {
    return { entries: this.cache.size, bytes: this.cacheBytes };
  }

  async loadSeries(
    fields: string[],
    startElapsedMs: number,
    endElapsedMs: number,
    buckets: number,
    thresholdSeconds: number | null
  ): Promise<SessionSeriesResult> {
    return this.load(
      `series:${fields.join('\u0000')}`,
      {
        kind: 'series',
        sessionId: this.metadata.summary.sessionId,
        snapshot: this.metadata.snapshot,
        range: { startElapsedMs, endElapsedMs },
        fields,
        buckets,
        gapPolicy: { thresholdSeconds },
      },
      (result): result is SessionSeriesResult => result.kind === 'series'
    );
  }

  async loadSamples(request: Omit<SessionSamplesRequest, 'sessionId' | 'snapshot'>): Promise<SessionSamplesResult> {
    return this.load(
      request.mode === 'nearest'
        ? `exact:${request.fields.join('\u0000')}`
        : `samples:${request.fields.join('\u0000')}`,
      { ...request, sessionId: this.metadata.summary.sessionId, snapshot: this.metadata.snapshot },
      (result): result is SessionSamplesResult => result.kind === 'samples'
    );
  }

  async loadEvents(request: Omit<SessionEventsRequest, 'sessionId' | 'snapshot'>): Promise<SessionEventsResult> {
    return this.load(
      'events',
      { ...request, sessionId: this.metadata.summary.sessionId, snapshot: this.metadata.snapshot },
      (result): result is SessionEventsResult => result.kind === 'events'
    );
  }

  invalidate() {
    for (const channel of new Set([...this.generations.keys(), ...this.activeAbort.keys()])) {
      this.generations.set(channel, (this.generations.get(channel) ?? 0) + 1);
      this.activeAbort.get(channel)?.abort();
      this.activeAbort.delete(channel);
    }
    this.cache.clear();
    this.cacheBytes = 0;
  }

  dispose() {
    this.disposed = true;
    this.invalidate();
  }

  private trimCache() {
    while (this.cache.size > MAX_CACHE_ENTRIES || this.cacheBytes > MAX_CACHE_BYTES) {
      const first = this.cache.keys().next().value as string | undefined;
      if (!first) break;
      const removed = this.cache.get(first);
      this.cache.delete(first);
      this.cacheBytes -= removed?.bytes ?? 0;
    }
  }

  private async load<T extends SessionDataResult>(
    channel: string,
    request: SessionDataRequest,
    isExpected: (result: SessionDataResult) => result is T
  ): Promise<T> {
    if (this.disposed) throw new Error('Session controller is disposed');
    const key = JSON.stringify(request);
    const cached = this.cache.get(key);
    if (cached && isExpected(cached.value)) {
      this.cache.delete(key);
      this.cache.set(key, cached);
      return cached.value;
    }
    const previousAbort = this.activeAbort.get(channel);
    if (previousAbort) {
      reportFrontendPerformance('session-data-abort', { channel });
      previousAbort.abort();
    }
    const abort = new AbortController();
    this.activeAbort.set(channel, abort);
    const generation = (this.generations.get(channel) ?? 0) + 1;
    this.generations.set(channel, generation);
    try {
      const result = await this.client.request(request, abort.signal);
      if (this.disposed || generation !== this.generations.get(channel) || !isExpected(result))
        throw new DOMException('Superseded session-data request', 'AbortError');
      const bytes = sessionDataDecodedBytes(result) ?? new TextEncoder().encode(JSON.stringify(result)).byteLength;
      this.cache.set(key, { value: result, bytes });
      this.cacheBytes += bytes;
      this.trimCache();
      return result;
    } finally {
      if (this.activeAbort.get(channel) === abort) this.activeAbort.delete(channel);
    }
  }
}

export const SESSION_REVIEW_CACHE_LIMITS = { entries: MAX_CACHE_ENTRIES, bytes: MAX_CACHE_BYTES } as const;
