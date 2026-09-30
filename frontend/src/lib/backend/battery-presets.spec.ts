import { describe, expect, it } from 'vitest';
import { applyBatteryPatch, batteryPresets } from '../../../backend-mock/battery-presets';
describe('shared battery preset mock contract', () => {
  it('selects atomically, preserves overrides, clears counts and rejects mixed input', () => {
    expect(batteryPresets).toHaveLength(52);
    const p = applyBatteryPatch({ motor: 'keep' }, { batteryPresetId: 'floatwheel-adv2-samsung-50s' });
    expect(p).toMatchObject({
      motor: 'keep',
      seriesCount: 20,
      parallelCount: 2,
      batteryCapacityAh: 10,
      batteryCellProfileId: 'samsung-50s',
    });
    const cleared = applyBatteryPatch(p, { batteryCapacityAh: null });
    expect(cleared.batteryPresetId).toBe(p.batteryPresetId);
    expect(applyBatteryPatch(cleared, { batteryPresetId: p.batteryPresetId }).batteryCapacityAh).toBe(10);
    expect(applyBatteryPatch(p, { parallelCount: 3 })).toMatchObject({
      batteryPresetId: null,
      batteryCapacityAh: null,
    });
    expect(applyBatteryPatch(p, { batteryCellProfileId: 'generic-linear' })).toMatchObject({
      batteryPresetId: null,
      batteryCapacityAh: 10,
    });
    for (const patch of [
      { batteryPresetId: 'unknown' },
      { batteryPresetId: p.batteryPresetId, batteryCapacityAh: null },
      { batteryCapacityAh: 201 },
    ])
      expect(() => applyBatteryPatch(p, patch)).toThrow();
  });
});
