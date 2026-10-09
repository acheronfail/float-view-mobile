import type { ActiveConnection, AlertReason, LiveSample, StopCondition } from '$lib/backend/contract';
import { telemetryState } from './telemetry';

export function stopReasonLabel(reason: StopCondition | null | undefined): string {
  const labels: Record<StopCondition, string> = {
    none: 'None',
    pitch: 'Pitch limit',
    roll: 'Roll limit',
    switchHalf: 'Footpad partially released',
    switchFull: 'Footpad released',
    reverseStop: 'Reverse stop',
    quickStop: 'Quick stop',
    unknown: 'Unknown',
  };
  return reason == null ? 'Unavailable' : labels[reason];
}

export function beepReasonLabel(reason: AlertReason | null | undefined): string {
  return reason === 'sensors' ? 'Footpad contact lost' : telemetryState(reason);
}

type StatusSample = Pick<
  LiveSample,
  'packageState' | 'stopCondition' | 'setpointAdjustment' | 'wheelslip' | 'charging' | 'firmwareFault'
>;

export function liveStatus(
  state: ActiveConnection['state'] | undefined,
  sample: StatusSample | null | undefined,
  charging = sample?.charging === true
): { display: string; safetyStateActive: boolean } {
  if (state !== 'ready') {
    return { display: state?.replaceAll('_', ' ') ?? 'Connecting', safetyStateActive: false };
  }

  // The firmware retains beep reasons after the beep ends. They cannot establish an active warning.
  if (sample?.firmwareFault) {
    return { display: `Firmware fault · ${telemetryState(sample.firmwareFault)}`, safetyStateActive: true };
  }
  if (charging) return { display: 'Charging', safetyStateActive: false };

  if (!sample) return { display: 'Connected', safetyStateActive: false };

  if (sample.packageState === 'running') {
    const conditions: string[] = [];
    if (sample.wheelslip) conditions.push('Wheelslip');
    const adjustment = sample.setpointAdjustment;
    if (adjustment && adjustment !== 'none' && adjustment !== 'centering') {
      conditions.push(adjustment === 'reverseStop' ? 'Reverse stop' : `Pushback · ${telemetryState(adjustment)}`);
    }
    return { display: ['Riding', ...conditions].join(' · '), safetyStateActive: conditions.length > 0 };
  }

  // Stop reasons and setpoint adjustments can remain populated while disengaged.
  if (sample.packageState === 'ready') {
    const reason = sample.stopCondition;
    return {
      display: reason && reason !== 'none' ? `Stopped · ${stopReasonLabel(reason)}` : 'Ready',
      safetyStateActive: false,
    };
  }
  return {
    display:
      sample.packageState === 'disabled'
        ? 'Balancing disabled'
        : sample.packageState === 'startup'
          ? 'Starting up'
          : 'Connected',
    safetyStateActive: false,
  };
}
