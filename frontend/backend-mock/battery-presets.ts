// Development backend reads the service-owned catalog; no browser-owned pack policy.
import { readFileSync } from 'node:fs';
import type { BatteryPreset, BoardProfileValues } from '../src/lib/backend/contract.ts';
export const batteryPresetCatalogVersion = 'packs-2026-09-30-v1';
export const batteryPresets: BatteryPreset[] = JSON.parse(
  readFileSync(new URL('../../backends/core/product-store/src/battery-presets.json', import.meta.url), 'utf8')
);
export function applyBatteryPatch(current: BoardProfileValues, patch: BoardProfileValues): BoardProfileValues {
  const fields = [
    'seriesCount',
    'parallelCount',
    'batteryCellProfileId',
    'batteryCapacityAh',
    'batteryMinimumCellVoltageV',
  ];
  let values = { ...current, ...patch };
  if (patch.batteryPresetId != null) {
    const preset = batteryPresets.find((p) => p.id === patch.batteryPresetId);
    if (!preset || fields.some((key) => key in patch)) throw new Error('Invalid battery selection');
    values = {
      ...values,
      seriesCount: preset.seriesCount,
      parallelCount: preset.parallelCount,
      batteryCellProfileId: preset.batteryCellProfileId,
      batteryCapacityAh: preset.nominalCapacityAh,
      batteryMinimumCellVoltageV: null,
    };
  } else {
    if (
      ['seriesCount', 'parallelCount'].some((key) => key in patch && patch[key] !== current[key]) &&
      !('batteryCapacityAh' in patch)
    )
      values.batteryCapacityAh = null;
    const preset = batteryPresets.find((p) => p.id === values.batteryPresetId);
    if (
      preset &&
      (preset.seriesCount !== values.seriesCount ||
        preset.parallelCount !== values.parallelCount ||
        preset.batteryCellProfileId !== values.batteryCellProfileId)
    )
      values.batteryPresetId = null;
  }
  for (const [key, max] of [
    ['seriesCount', 36],
    ['parallelCount', 20],
  ] as const) {
    const value = values[key];
    if (value != null && (!Number.isInteger(value) || value < 1 || value > max)) throw new Error('Invalid pack count');
  }
  const capacity = values.batteryCapacityAh;
  if (
    capacity != null &&
    (typeof capacity !== 'number' || !Number.isFinite(capacity) || capacity < 0.1 || capacity > 200)
  )
    throw new Error('Invalid pack capacity');
  return values;
}
