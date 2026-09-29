import { expect, it } from 'vitest';
import { validateSessionDataRequest, validateSessionDataResult, type SessionMarkersRequest } from './session-data';
const snapshot = {
  historyRevision: 0,
  committedSampleSequence: 0,
  committedEventSequence: 3,
  committedElapsedMs: 1000,
  committedLocationSequence: 0,
  committedLocationElapsedMs: 0,
};
const request: SessionMarkersRequest = {
  kind: 'markers',
  sessionId: 'one',
  snapshot,
  range: { startElapsedMs: 0, endElapsedMs: 1000 },
  limit: 2,
};
const marker = {
  sequence: 1,
  elapsedMs: 500,
  observedAtMs: 500,
  operationId: '00000000-0000-4000-8000-000000000001',
  location: null,
};
it('validates bounded marker-only pages and exact snapshot identity', () => {
  expect(validateSessionDataRequest(request)).toEqual(request);
  expect(() => validateSessionDataRequest({ ...request, limit: 257 })).toThrow();
  const result = { ...request, markers: [marker], nextCursor: null };
  expect(validateSessionDataResult(result, request)).toEqual(result);
  for (const invalid of [
    { ...result, markers: [marker, marker] },
    { ...result, markers: [{ ...marker, sequence: 4 }] },
    { ...result, markers: [{ ...marker, elapsedMs: 1001 }] },
    { ...result, markers: [{ ...marker, location: { latitude: 200 } }] },
    { ...result, snapshot: { ...snapshot, historyRevision: 1 } },
  ])
    expect(() => validateSessionDataResult(invalid, request)).toThrow();
});
