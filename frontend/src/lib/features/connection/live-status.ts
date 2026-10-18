import type { ActiveConnection, AlertReason, LiveSample, StopCondition } from '$lib/backend/contract';
import { telemetryState } from './telemetry';

export function stopReasonLabel(reason: StopCondition | null | undefined): string {
  const labels: Record<StopCondition, string> = {
    none: 'None',
    pitch: 'Pitch limit',
    roll: 'Roll limit',
    switchHalf: 'One footpad',
    switchFull: 'Both footpads',
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
  | 'packageState'
  | 'stopCondition'
  | 'setpointAdjustment'
  | 'wheelslip'
  | 'charging'
  | 'firmwareFault'
  | 'fatalErrorLatched'
  | 'lastFirmwareFault'
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
  const fault = firmwareFaultWarning(sample);
  if (fault) return { display: fault.title, safetyStateActive: true };
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

/** A retained beep reason alone cannot establish an active or latched warning. */
export function firmwareFaultWarning(
  sample: Pick<LiveSample, 'firmwareFault' | 'fatalErrorLatched' | 'lastFirmwareFault'> | null | undefined
): { title: string; description: string } | null {
  if (!sample?.firmwareFault && sample?.fatalErrorLatched !== true) return null;
  const reason = sample.firmwareFault ?? sample.lastFirmwareFault?.reason;
  const code = sample.lastFirmwareFault?.code;
  const label = reason
    ? telemetryState(reason) + (reason === 'unknown' && code != null ? ` (code ${code})` : '')
    : 'Reason unavailable';
  return {
    title: `Firmware fault · ${label}`,
    description: sample.firmwareFault
      ? 'Stop riding and investigate this controller fault.'
      : 'Fault ended; fatal error latched. Stop riding and investigate before power cycling or using Clear fatal error in Board actions.',
  };
}
