import { describe, expect, it } from 'vitest';
import { beepReasonLabel, firmwareFaultWarning, hasFirmwareFaults, liveStatus, stopReasonLabel } from './live-status';

const sample = {
  packageState: 'ready' as const,
  stopCondition: 'switchHalf' as const,
  setpointAdjustment: 'error' as const,
  alertReason: 'sensors' as const,
  wheelslip: false,
  charging: false,
  firmwareFault: null,
  fatalErrorLatched: false,
  lastFirmwareFault: null,
};

describe('live status presentation', () => {
  it('keeps a recovered fault visible until the fatal latch clears', () => {
    const recovered = {
      ...sample,
      fatalErrorLatched: true,
      lastFirmwareFault: { code: 4, reason: 'absoluteOverCurrent' as const },
    };
    expect(firmwareFaultWarning(recovered)?.title).toContain('Absolute Over Current');
    expect(firmwareFaultWarning(recovered)?.description).toContain('latched');
    expect(liveStatus('ready', recovered).safetyStateActive).toBe(true);
    const cleared = { ...recovered, fatalErrorLatched: false, alertReason: 'firmwareFault' as const };
    expect(firmwareFaultWarning(cleared)).toBeNull();
    expect(firmwareFaultWarning({ ...sample, fatalErrorLatched: true })?.title).toContain('Reason unavailable');
    expect(
      firmwareFaultWarning({ ...recovered, lastFirmwareFault: { code: 255, reason: 'unknown' } })?.title
    ).toContain('code 255');
  });

  it('offers history for current warnings and retained occurrences only', () => {
    expect(hasFirmwareFaults(null)).toBe(false);
    expect(hasFirmwareFaults({ ...sample, faultHistory: [] })).toBe(false);
    expect(hasFirmwareFaults({ ...sample, fatalErrorLatched: true, faultHistory: [] })).toBe(true);
    expect(hasFirmwareFaults({ ...sample, firmwareFault: 'driver', faultHistory: [] })).toBe(true);
    expect(
      hasFirmwareFaults({
        ...sample,
        faultHistory: [{ code: 3, reason: 'driver', elapsedMs: 0, observedElapsedMs: 0 }],
      })
    ).toBe(true);
    expect(firmwareFaultWarning({ ...sample, fatalErrorLatched: true })?.description).toContain('VESC Tool');
    expect(firmwareFaultWarning({ ...sample, fatalErrorLatched: true })?.description).not.toContain('fault has ended');
  });

  it('clears the stop explanation on engagement despite retained stop and beep reasons', () => {
    expect(liveStatus('ready', sample)).toEqual({
      display: 'Stopped · One footpad',
      safetyStateActive: false,
    });
    const riding = { ...sample, packageState: 'running' as const, setpointAdjustment: 'none' as const };
    expect(liveStatus('ready', riding)).toEqual({ display: 'Riding', safetyStateActive: false });
    expect(liveStatus('ready', { ...riding, stopCondition: 'none' })).toEqual({
      display: 'Riding',
      safetyStateActive: false,
    });
    expect(liveStatus('ready', { ...sample, stopCondition: 'none' })).toEqual({
      display: 'Ready',
      safetyStateActive: false,
    });
  });

  it('shows current wheelslip and pushback only while riding', () => {
    expect(
      liveStatus('ready', { ...sample, packageState: 'running', wheelslip: true, setpointAdjustment: 'lowVoltage' })
    ).toEqual({ display: 'Riding · Wheelslip · Pushback · Low Voltage', safetyStateActive: true });
    expect(liveStatus('ready', { ...sample, packageState: 'running', setpointAdjustment: 'centering' })).toEqual({
      display: 'Riding',
      safetyStateActive: false,
    });
  });

  it('prioritizes active firmware faults and connection state', () => {
    const fault = { ...sample, firmwareFault: 'underVoltage' as const };
    expect(liveStatus('ready', fault)).toEqual({ display: 'Firmware fault · Under Voltage', safetyStateActive: true });
    expect(liveStatus('reconnecting', fault)).toEqual({ display: 'reconnecting', safetyStateActive: false });
  });

  it('handles unavailable data and package lifecycle states', () => {
    expect(liveStatus('ready', null).display).toBe('Connected');
    expect(liveStatus('ready', { ...sample, packageState: null }).display).toBe('Connected');
    expect(liveStatus('ready', { ...sample, packageState: 'startup' }).display).toBe('Starting up');
    expect(liveStatus('ready', { ...sample, packageState: 'disabled' }).display).toBe('Balancing disabled');
    expect(liveStatus('ready', { ...sample, charging: true }).display).toBe('Charging');
  });

  it('describes historical footpad reasons clearly', () => {
    expect(beepReasonLabel('sensors')).toBe('Footpad contact lost');
    expect(beepReasonLabel(null)).toBe('Unavailable');
    expect(beepReasonLabel('none')).toBe('None');
    expect(stopReasonLabel('switchHalf')).toBe('One footpad');
    expect(stopReasonLabel('switchFull')).toBe('Both footpads');
  });
});
