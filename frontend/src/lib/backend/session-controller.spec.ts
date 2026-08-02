import { describe, expect, it } from 'vitest';
import { SessionReviewController } from './session-controller';
import { SessionDataClient } from './session-data-client';
import type { SessionMetadata } from './contract';

const metadata: SessionMetadata = {
  summary: {
    sessionId: 'controller-session',
    deviceHistoryId: 'history',
    displayName: 'Controller session',
    startedAt: '2026-01-01T00:00:00Z',
    endedAt: '2026-01-01T00:01:00Z',
    state: 'completed',
    integrity: 'complete',
    sampleCount: 2,
    eventCount: 0,
    ridingElapsedMs: 60_000,
    totalElapsedMs: 60_000,
    distanceM: 1,
    maxSpeedMps: 1,
    maxDutyCycle: 0.2,
  },
  fields: [{ name: 'speedKph', source: 'stable' }],
  eventCount: 0,
  recordedSampleCount: 2,
  storedSampleCount: 2,
  prune: { targetResolutionSeconds: null, recordedSampleCount: 2, storedSampleCount: 2, policy: null },
  snapshot: { historyRevision: 1, committedSampleSequence: 1, committedEventSequence: 0, committedElapsedMs: 60_000 },
};

function clientWithResponses() {
  let calls = 0;
  const client = new SessionDataClient(
    { webSocketUrl: '/api/ws', sessionDataUrl: '/api/session-data', sessionToken: 'token' },
    false,
    'http://127.0.0.1/',
    async () => {
      calls += 1;
      return new Response(
        JSON.stringify({
          kind: 'series',
          sessionId: metadata.summary.sessionId,
          snapshot: metadata.snapshot,
          range: { startElapsedMs: 0, endElapsedMs: 60_000 },
          gapPolicy: { thresholdSeconds: null },
          fields: metadata.fields,
          columns: { speedKph: { elapsedMs: [0], sequence: [0], values: [1], generations: [0] } },
        }),
        { headers: { 'Content-Type': 'application/json' } }
      );
    }
  );
  return { client, calls: () => calls };
}

describe('session review controller', () => {
  it('caches an identical series request and bounds entries/bytes', async () => {
    const { client, calls } = clientWithResponses();
    const controller = new SessionReviewController(metadata, client);
    await controller.loadSeries(['speedKph'], 0, 60_000, 16, null);
    await controller.loadSeries(['speedKph'], 0, 60_000, 16, null);
    expect(calls()).toBe(1);
    expect(controller.cacheLimits).toEqual({ entries: 32, bytes: 8 * 1024 * 1024 });
    controller.dispose();
  });

  it('rejects a settled response after disposal', async () => {
    let release!: () => void;
    const pending = new Promise<Response>((resolve) => {
      release = () => resolve(new Response('{}', { status: 500 }));
    });
    const client = new SessionDataClient(
      { webSocketUrl: '/api/ws', sessionDataUrl: '/api/session-data' },
      false,
      'http://127.0.0.1/',
      () => pending
    );
    const controller = new SessionReviewController(metadata, client);
    const request = controller.loadSeries(['speedKph'], 0, 60_000, 16, null);
    controller.dispose();
    release();
    await expect(request).rejects.toThrow();
  });
});
