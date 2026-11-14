import { render } from 'svelte/server';
import { describe, expect, it, vi } from 'vitest';
import { emptySpeedCalibration } from '$lib/backend/contract';
import type { SpeedCalibrationState } from '$lib/backend/contract';
const backend = vi.hoisted(() => ({ snapshot: {} as Record<string, unknown>, speedCalibration: vi.fn() }));
vi.mock('$lib/backend/state.svelte', () => ({ backendState: backend }));
import SpeedCalibration from './SpeedCalibration.svelte';
function screen(
  patch: Partial<SpeedCalibrationState>,
  units: 'metric' | 'imperial' = 'metric',
  speedKph: number | null = 18
) {
  backend.snapshot = {
    speedCalibration: { ...emptySpeedCalibration, ...patch },
    permissions: [{ permission: 'location', status: 'granted', servicesEnabled: true }],
    settings: { unitPreference: units },
    activeConnection: { state: 'ready', latestSample: { speedKph } },
    location: { status: 'disabled', backgroundCapture: 'available' },
  };
  return render(SpeedCalibration).body.replace(/\s+/g, ' ');
}
describe('speed calibration', () => {
  it('keeps a prominent recommendation and manual guidance when application is unsupported', () => {
    const html = screen({
      status: 'ready',
      startingDiameterMm: 280,
      recommendedDiameterMm: 268.8,
      applyBlockReason: 'unsupportedSetup',
    });
    expect(html).toContain('268.8');
    expect(html).toContain('VESC Tool');
    expect(html).toContain('Apply is unavailable for this setup.');
    expect(html).not.toMatch(/<button[^>]*>Apply /);
    expect(backend.speedCalibration).not.toHaveBeenCalled();
  });
  it('uses existing location access with route recording disabled', () => {
    const html = screen({ currentDiameterMm: 280 });
    expect(html).toMatch(/<button[^>]*>Start calibration<\/button>/);
    expect(html).not.toContain('Allow location');
  });
  it('shows insufficient evidence without a fabricated result', () => {
    const html = screen({ status: 'insufficient' });
    expect(html).toContain('Calibration ended without a reliable diameter estimate.');
    expect(html).not.toContain('Recommended wheel diameter');
  });
  it('requires stopping before an otherwise supported apply', () => {
    const html = screen({ status: 'ready', recommendedDiameterMm: 269, applyBlockReason: 'stopRiding' });
    expect(html).toContain('Apply is available when the board is idle');
    expect(html).toMatch(/<button[^>]* disabled[^>]*>Apply 269.0 mm/);
  });
});

it.each([
  ['waitingGps', 'Waiting for GPS'],
  ['waitingBoard', 'Waiting for board'],
  ['lowSpeed', 'Speed too low'],
  ['gpsUncertain', 'GPS speed uncertain'],
  ['speedChanging', 'Keep speed steady'],
  ['speedMismatch', 'Speed readings differ'],
  ['needsMoreMeasurements', 'Measuring'],
  ['inconsistentMeasurements', 'Measuring'],
  ['ready', 'Measuring'],
] as const)('explains %s using app status only', (measurementStatus, copy) => {
  const html = screen({ status: 'collecting', measurementStatus });
  expect(html).toContain(copy);
  for (const coaching of [
    'Put your phone away',
    'Ride steadily',
    'Motor poles',
    'gearing',
    'clear view',
    'Keep Float View',
  ])
    expect(html).not.toContain(coaching);
});
it('describes only the comparison before starting', () => {
  const html = screen({ currentDiameterMm: 280 });
  expect(html).toContain('We’ll compare GPS and board speed to recommend a wheel diameter.');
  expect(html).not.toContain('Motor poles');
  expect(html).not.toContain('flat route');
});

it('shows approximate progress beyond the minimum without promising completion', () => {
  const html = screen({
    status: 'collecting',
    measurementStatus: 'needsMoreMeasurements',
    acceptedSamples: 42,
    progressPercent: 82,
  });
  expect(html).toContain('Measuring');
  expect(html).not.toMatch(/>42</);
  expect(html).toContain('Approximate progress');
  expect(html).toContain('role="progressbar"');
  expect(html).toContain('aria-valuenow="82"');
  expect(html).toContain('aria-valuetext="Approximately 82% complete"');
  expect(html).not.toContain('Measurements kept');
  expect(html).not.toContain('Comparisons pause');
  expect(html).not.toContain('Finish calibration');
  expect(html).not.toContain('/ 30');
  expect(html).not.toContain('<progress');
  expect(html).toContain('role="status"');
  expect(html).toContain('>Cancel<');
});

it.each([
  ['metric', '18.0', '7.2 km/h'],
  ['imperial', '11.2', '4.5 mph'],
] as const)('shows labelled %s speed, lower threshold without a duration target', (units, speed, minimum) => {
  const html = screen({ status: 'collecting', measurementStatus: 'lowSpeed' }, units);
  expect(html).toContain('Board speed');
  expect(html).toMatch(new RegExp(`data-calibration-speed="">${speed.replace('.', '\\.')}<`));
  expect(html).toContain(minimum);
  expect(html).not.toContain('Time remaining');
  expect(html).not.toContain('3:00');
  expect(html).not.toContain('Finishes automatically');
});
it('keeps unavailable board speed unavailable', () => {
  const html = screen({ status: 'collecting', measurementStatus: 'waitingBoard' }, 'metric', null);
  expect(html).toMatch(/data-calibration-speed="">—</);
});

it.each([
  ['gpsUncertain', 'GPS speed uncertain'],
  ['lowSpeed', 'Speed too low'],
  ['ready', 'Measuring'],
] as const)('keeps %s visible with Finish available', (measurementStatus, statusText) => {
  const html = screen({
    status: 'refining',
    measurementStatus,
    recommendedDiameterMm: 268.8,
    progressPercent: 100,
  });
  expect(html).toContain(statusText);
  expect(html).not.toContain('Result ready');
  expect(html).toContain('More measurements may improve precision.');
  expect(html).toContain('Finish calibration');
  if (measurementStatus === 'lowSpeed') expect(html).toContain('Minimum 7.2 km/h');
  expect(html).not.toContain('Recommended wheel diameter');
  expect(html).not.toContain('aria-label="Calibration progress"');
  expect(html).not.toMatch(/<button[^>]*>Apply /);
});
