import { describe, expect, it } from 'vitest';
import { emptyEstimate } from '../../../../backend-mock/battery';
import { batteryPresentation } from './battery';

describe('battery display source', () => {
  const unconfigured = { ...emptyEstimate(), unavailableReason: 'not_configured' as const };
  it('prefers and rounds the authoritative estimate', () => {
    expect(batteryPresentation({ ...unconfigured, level: 0.195 }, 0.8, true)).toEqual({
      percent: 20,
      needsSetup: false,
      label: 'Estimated battery level',
    });
  });
  it('uses reported charge only for an unconfigured saved board', () => {
    expect(batteryPresentation(unconfigured, 0.8, true)).toEqual({
      percent: 80,
      needsSetup: true,
      label: 'Controller-reported battery level',
    });
    expect(batteryPresentation(unconfigured, 0, true).percent).toBe(0);
    expect(batteryPresentation(unconfigured, 0.8, false).percent).toBeNull();
    for (const unavailableReason of ['series_mismatch', 'missing_voltage', 'not_applicable'] as const) {
      expect(batteryPresentation({ ...unconfigured, unavailableReason }, 0.8, true)).toMatchObject({
        percent: null,
        needsSetup: false,
      });
    }
  });
  it('keeps setup actionable without fabricating a reported value', () => {
    for (const value of [null, undefined, NaN, Infinity, -0.1, 1.1]) {
      expect(batteryPresentation(unconfigured, value, true)).toMatchObject({ percent: null, needsSetup: true });
    }
  });
});
