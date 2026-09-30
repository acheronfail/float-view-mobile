// Development backend only. Uses the pinned Rust test fixture, never browser code.
import { readFileSync } from 'node:fs';
import type { ActiveConnection, BatteryEstimate, BoardProfileValues } from '../src/lib/backend/contract.ts';
const data = JSON.parse(
  readFileSync(new URL('../../backends/core/product-store/tests/battery-profile-data.json', import.meta.url), 'utf8')
) as {
  catalogVersion: string;
  profiles: { id: string; label: string; curves: { currentA: number; volts: number[] }[] }[];
};
export const batteryCellProfiles = data.profiles.map(({ id, label }) => ({ id, label }));
export const emptyEstimate = (): BatteryEstimate => ({
  level: null,
  profileId: null,
  seriesCount: null,
  sampleSequence: null,
  profileRevision: null,
  catalogVersion: data.catalogVersion,
  mode: 'unavailable',
  unavailableReason: 'not_applicable',
});
const inverse = (v: number[], x: number) => {
  if (x >= v[0]!) return 1;
  if (x <= v[10]!) return 0;
  const i = v.findIndex((_, i) => i < 10 && x >= v[i + 1]!);
  return i < 0 ? 0 : 1 - (i + (v[i]! - x) / (v[i]! - v[i + 1]!)) / 10;
};
const voltage = (v: number[], q: number) => {
  const x = (1 - q) * 10,
    i = Math.min(9, Math.floor(x));
  return v[i]! + (v[i + 1]! - v[i]!) * (x - i);
};
export class MockBattery {
  private key = '';
  private sequence = -1;
  private time = 0;
  private current: number | null = null;
  private last = emptyEstimate();
  update(active: ActiveConnection | null, profile?: { revision: number; values: BoardProfileValues }): BatteryEstimate {
    if (!active) {
      this.key = '';
      return emptyEstimate();
    }
    const c = profile?.values ?? {},
      s = active.latestSample;
    const key = JSON.stringify([
      active.deviceId,
      active.trackingMode,
      active.deviceKind,
      c.batteryCellProfileId,
      c.seriesCount,
      c.parallelCount,
      c.batteryMinimumCellVoltageV,
    ]);
    if (key !== this.key) {
      this.key = key;
      this.sequence = -1;
      this.current = null;
    }
    if (active.state !== 'ready') this.current = null;
    if (s && s.sequence === this.sequence && profile?.revision === this.last.profileRevision) return this.last;
    if (s && s.sequence !== this.sequence) {
      const amps = s.batteryCurrentA != null && s.batteryCurrentA > 0 ? s.batteryCurrentA : null;
      this.current =
        amps == null || this.current == null || s.elapsedMs - this.time > 5000
          ? amps
          : this.current + (1 - Math.exp(-(s.elapsedMs - this.time) / 2000)) * (amps - this.current);
      this.sequence = s.sequence;
      this.time = s.elapsedMs;
    }
    const e = emptyEstimate();
    e.sampleSequence = s?.sequence ?? null;
    e.profileRevision = profile?.revision ?? null;
    if (active.trackingMode === 'saved' && active.deviceKind !== 'bms') {
      e.profileId = c.batteryCellProfileId ?? null;
      e.seriesCount = c.seriesCount ?? null;
      e.unavailableReason = 'not_configured';
      const p = data.profiles.find((p) => p.id === c.batteryCellProfileId);
      if (p && c.seriesCount && c.parallelCount) {
        if (active.currentLimits?.batterySeriesCells && active.currentLimits.batterySeriesCells !== c.seriesCount) {
          e.seriesCount = null;
          e.unavailableReason = 'series_mismatch';
        } else if (s?.packVoltageV == null || !Number.isFinite(s.packVoltageV) || s.packVoltageV <= 0)
          e.unavailableReason = 'missing_voltage';
        else {
          const minimum = c.batteryMinimumCellVoltageV ?? 3;
          const adjusted = p.curves.map((curve) => {
            const qMin = inverse(curve.volts, minimum);
            return Array.from({ length: 11 }, (_, i) =>
              i === 0 ? 4.2 : i === 10 ? minimum : voltage(curve.volts, qMin + (1 - qMin) * (1 - i / 10))
            );
          });
          let knots = adjusted[0]!;
          e.mode = 'voltage_only';
          const amps = (this.current ?? 0) / c.parallelCount;
          if (amps > 0 && p.curves.length > 1) {
            const load = p.curves[1]!.currentA,
              fraction = Math.min(1, amps / load);
            knots = knots.map((v, i) => v + fraction * (adjusted[1]![i]! - v));
            e.mode = amps > load ? 'load_clamped' : 'load_compensated';
          }
          e.level = inverse(knots, s.packVoltageV / c.seriesCount);
          e.unavailableReason = null;
        }
      }
    }
    this.last = e;
    return e;
  }
}
