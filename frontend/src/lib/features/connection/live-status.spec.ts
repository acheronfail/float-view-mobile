import { describe, expect, it } from 'vitest';
import { beepReasonLabel, liveStatus, stopReasonLabel } from './live-status';

const sample = {
  packageState: 'ready' as const,
  stopCondition: 'switchHalf' as const,
  setpointAdjustment: 'error' as const,
  alertReason: 'sensors' as const,
  wheelslip: false,
  charging: false,
  firmwareFault: null,
};

describe('live status presentation', () => {
  it('clears the stop explanation on engagement despite retained stop and beep reasons', () => {
    expect(liveStatus('ready', sample)).toEqual({
      display: 'Stopped · Footpad partially released',
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
    expect(stopReasonLabel('switchHalf')).toBe('Footpad partially released');
    expect(stopReasonLabel('switchFull')).toBe('Footpad released');
  });
});
