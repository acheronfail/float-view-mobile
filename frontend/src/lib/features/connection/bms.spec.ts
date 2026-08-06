import { describe, expect, it } from 'vitest';
import type { BmsState } from '$lib/backend/contract';
import { bmsFreshness, bmsMenuVisible, bmsRouteLabel } from './bms';

const base: BmsState = { status: 'discovering', connection: null, candidates: [], latestSnapshot: null };

describe('BMS presentation policy', () => {
  it('shows the action only after an identity is known', () => {
    expect(bmsMenuVisible(base)).toBe(false);
    expect(bmsMenuVisible({ ...base, status: 'not_found' })).toBe(false);
    expect(bmsMenuVisible({ ...base, status: 'attached', connection: { kind: 'can', canId: 42 } })).toBe(true);
    expect(bmsMenuVisible({ ...base, status: 'unavailable', connection: { kind: 'direct' } })).toBe(true);
    expect(
      bmsMenuVisible({
        ...base,
        status: 'ambiguous',
        candidates: [{ connection: { kind: 'can', canId: 3 }, hardwareName: 'VBMS', vescVersion: '6.00' }],
      })
    ).toBe(true);
  });

  it('formats routes and freshness without changing the observation time', () => {
    expect(bmsRouteLabel({ kind: 'direct' })).toBe('Direct Bluetooth endpoint');
    expect(bmsRouteLabel({ kind: 'can', canId: 42 })).toBe('CAN 42');
    expect(bmsFreshness('2026-08-06T00:00:00.000Z', Date.parse('2026-08-06T00:01:05.000Z'))).toBe('1m ago');
  });
});
