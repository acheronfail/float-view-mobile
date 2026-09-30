// Development backend only. Uses service-owned curve fixtures, never browser estimation.
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
  method: null,
  provisional: false,
  unavailableReason: 'not_applicable',
});
const clamp = (v: number) => Math.max(0, Math.min(1, v));
const inverse = (v: number[], x: number) => {
  if (x >= v[0]!) return 1;
  if (x <= v[10]!) return 0;
  const i = v.findIndex((_, i) => i < 10 && x >= v[i + 1]!);
  return i < 0 ? 0 : 1 - (i + (v[i]! - x) / (v[i]! - v[i + 1]!)) / 10;
};
const display = (q: number, empty: number) => clamp((q - empty) / (1 - empty));
export class MockBattery {
  private key = '';
  private sequence = -1;
  private acceptedTime = -1;
  private reconnecting = false;
  private last = emptyEstimate();
  private charge: number | null = null;
  private pair: { time: number; current: number | null } | null = null;
  private settling: { since: number; min: number; max: number } | null = null;
  private provisional = true;
  private interrupt() {
    this.pair = null;
    this.settling = null;
    this.provisional = true;
  }
  update(active: ActiveConnection | null, profile?: { revision: number; values: BoardProfileValues }): BatteryEstimate {
    if (!active) {
      this.key = '';
      this.charge = null;
      this.interrupt();
      return emptyEstimate();
    }
    const c = profile?.values ?? {},
      s = active.latestSample;
    const key = JSON.stringify([
      active.deviceId,
      active.savedDeviceId,
      active.trackingMode,
      active.deviceKind,
      c.batteryCellProfileId,
      c.seriesCount,
      c.parallelCount,
      c.batteryCapacityAh ?? null,
      c.batteryMinimumCellVoltageV ?? 3,
    ]);
    const changed = key !== this.key;
    if (changed) {
      this.key = key;
      this.sequence = -1;
      this.acceptedTime = -1;
      this.charge = null;
      this.interrupt();
    }
    if (active.state !== 'ready') {
      this.reconnecting = true;
      this.interrupt();
    }
    const resumed = this.reconnecting && active.state === 'ready';
    if (resumed) {
      this.sequence = -1;
      this.acceptedTime = -1;
      this.reconnecting = false;
      this.interrupt();
    }
    if (!changed && s && (s.sequence <= this.sequence || s.elapsedMs < this.acceptedTime)) {
      this.last = { ...this.last, profileRevision: profile?.revision ?? null };
      return this.last;
    }
    if (s) {
      this.sequence = s.sequence;
      this.acceptedTime = s.elapsedMs;
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
        } else if (!s || s.packVoltageV == null || !Number.isFinite(s.packVoltageV) || s.packVoltageV <= 0)
          e.unavailableReason = 'missing_voltage';
        else {
          const current = s.batteryCurrentA != null && Number.isFinite(s.batteryCurrentA) ? s.batteryCurrentA : null;
          const amps = Math.max(0, current ?? 0) / c.parallelCount;
          let knots = p.curves[0]!.volts;
          e.mode = 'voltage_only';
          if (amps > 0 && p.curves.length > 1) {
            const last = p.curves.at(-1)!;
            e.mode = amps > last.currentA ? 'load_clamped' : 'load_compensated';
            for (let i = 1; i < p.curves.length; i++) {
              const low = p.curves[i - 1]!,
                high = p.curves[i]!;
              const f = clamp((amps - low.currentA) / (high.currentA - low.currentA));
              knots = low.volts.map((v, j) => v + f * (high.volts[j]! - v));
              if (amps <= high.currentA) break;
            }
          }
          const cell = s.packVoltageV / c.seriesCount,
            minimum = c.batteryMinimumCellVoltageV ?? 3;
          const empty = Math.min(1 - Number.EPSILON, inverse(p.curves[0]!.volts, minimum));
          let observation = inverse(knots, cell);
          if (amps === 0 && cell <= minimum) observation = Math.min(observation, empty);
          const time = s.elapsedMs;
          const pair = this.pair && time >= this.pair.time && time - this.pair.time <= 5000 ? this.pair : null;
          if (!pair) this.interrupt();
          const dt = pair ? (time - pair.time) / 1000 : 0;
          let q = this.charge ?? observation;
          const capacity = c.batteryCapacityAh;
          const integrated =
            capacity != null &&
            capacity >= 0.1 &&
            capacity <= 200 &&
            pair?.current != null &&
            current != null &&
            dt > 0;
          if (integrated) q = clamp(q - ((pair.current! + current!) * dt) / (2 * 3600 * capacity));
          const low = s.charging !== true && current != null && Math.abs(current) <= 0.1 * c.parallelCount;
          if (low) {
            const w = this.settling;
            if (w && Math.max(w.max, cell) - Math.min(w.min, cell) <= 0.01) {
              w.min = Math.min(w.min, cell);
              w.max = Math.max(w.max, cell);
            } else this.settling = { since: time, min: cell, max: cell };
          } else this.settling = null;
          const settled = this.settling != null && time - this.settling.since >= 30000;
          if (!integrated || settled) {
            const bound = 0.001 * dt * (1 - empty);
            q = clamp(
              q + Math.max(-bound, Math.min(bound, (observation - q) * (1 - Math.exp(-dt / (integrated ? 120 : 30)))))
            );
          }
          if (current == null || current < 0 || e.mode === 'load_clamped' || (p.curves.length === 1 && current > 0))
            this.provisional = true;
          if (settled && Math.abs(display(q, empty) - display(observation, empty)) <= 0.02) this.provisional = false;
          this.charge = q;
          this.pair = { time, current };
          e.level = display(q, empty);
          e.method = integrated ? 'charge_integrated' : 'voltage_smoothed';
          e.provisional = this.provisional;
          e.unavailableReason = null;
        }
      }
    }
    if (e.level === null) this.interrupt();
    this.last = e;
    return e;
  }
}
