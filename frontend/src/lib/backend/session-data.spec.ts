import { describe, expect, it } from 'vitest';
import {
  SessionDataProtocolError,
  readBoundedJson,
  validateSessionDataRequest,
  validateSessionDataResult,
} from './session-data';
import { MAX_SESSION_DATA_RESPONSE_BYTES } from './contract';

const snapshot = {
  historyRevision: 0,
  committedSampleSequence: 10,
  committedEventSequence: 2,
  committedElapsedMs: 1_000,
};
const range = { startElapsedMs: 0, endElapsedMs: 1_000 };

describe('session-data protocol', () => {
  it('validates bounded discriminated requests', () => {
    expect(
      validateSessionDataRequest({
        kind: 'series',
        sessionId: 'session-1',
        snapshot,
        range,
        fields: ['packVoltageV'],
        buckets: 32,
        gapPolicy: { thresholdSeconds: null },
      })
    ).toMatchObject({ kind: 'series', buckets: 32 });
    expect(() =>
      validateSessionDataRequest({
        kind: 'series',
        sessionId: 'session-1',
        snapshot,
        range,
        fields: Array.from({ length: 17 }, (_, index) => `field${index}`),
        buckets: 1,
        gapPolicy: { thresholdSeconds: null },
      })
    ).toThrow(SessionDataProtocolError);
  });

  it('rejects mismatched parallel arrays and identity', () => {
    const request = validateSessionDataRequest({
      kind: 'samples',
      sessionId: 'session-1',
      snapshot,
      range,
      fields: ['packVoltageV'],
      mode: 'window',
      limit: 2,
    });
    expect(() =>
      validateSessionDataResult(
        {
          kind: 'samples',
          sessionId: 'other',
          snapshot,
          range,
          mode: 'window',
          fields: [{ name: 'packVoltageV', unit: 'V', source: 'stable' }],
          elapsedMs: [0, 1],
          sequence: [1],
          values: { packVoltageV: [60, 59] },
          nextCursor: null,
        },
        request
      )
    ).toThrow(SessionDataProtocolError);
  });

  it('accepts a validated columnar series result', () => {
    const request = validateSessionDataRequest({
      kind: 'series',
      sessionId: 'session-1',
      snapshot,
      range,
      fields: ['packVoltageV'],
      buckets: 2,
      gapPolicy: { thresholdSeconds: 1 },
    });
    const result = validateSessionDataResult(
      {
        kind: 'series',
        sessionId: 'session-1',
        snapshot,
        range,
        gapPolicy: { thresholdSeconds: 1 },
        fields: [{ name: 'packVoltageV', unit: 'V', source: 'stable' }],
        columns: { packVoltageV: { elapsedMs: [0], sequence: [1], values: [60], generations: [0] } },
      },
      request
    );
    expect(result.kind).toBe('series');
  });

  it('counts decoded streamed bytes before parsing', async () => {
    const payload = JSON.stringify({ ok: true });
    await expect(
      readBoundedJson(
        new Response(
          new ReadableStream({
            start(controller) {
              controller.enqueue(new TextEncoder().encode(payload.slice(0, 3)));
              controller.enqueue(new TextEncoder().encode(payload.slice(3)));
              controller.close();
            },
          })
        )
      )
    ).resolves.toEqual({ ok: true });
    const oversized = new Uint8Array(MAX_SESSION_DATA_RESPONSE_BYTES + 1);
    await expect(readBoundedJson(new Response(oversized))).rejects.toThrow(SessionDataProtocolError);
  });
});
