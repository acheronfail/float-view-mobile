import { emptyEstimate } from '../../../../backend-mock/battery';
import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { LiveSample } from '../../backend/contract';
import { estimatedBatteryPercent } from './battery';
import GaugePower from './WidgetPower.svelte';

describe('battery power gauge', () => {
  it('rounds one estimate for text, gauge fill, color and accessibility', () => {
    const estimate = { ...emptyEstimate(), level: 0.1949, seriesCount: 20 };
    const sample = { packVoltageV: 72, batteryLevel: 0.8, charging: true } as LiveSample;
    const { body } = render(GaugePower, { props: { sample, estimate } });
    expect(estimatedBatteryPercent(estimate)).toBe(19);
    expect(body).toContain('19%');
    expect(body).toContain('aria-valuenow="19"');
    expect(body).toContain('19 percent');
    expect(body).toContain('width: 19%');
    expect(body).toContain('rgb(220, 38, 38)');
    expect(body).toContain('Charging');
    expect(body).toContain('3.60 V');
    expect(estimatedBatteryPercent({ ...estimate, level: 0.195 })).toBe(20);
    expect(estimatedBatteryPercent(null)).toBeNull();
  });
  it('does not substitute controller charge when the estimate is unavailable', () => {
    const sample = { packVoltageV: 72, batteryLevel: 0.8, charging: false } as LiveSample;
    const { body } = render(GaugePower, { props: { sample, estimate: emptyEstimate() } });
    expect(body).not.toMatch(/<output[^>]*>80%<\/output>/);
    expect(body).toContain('72.0 V');
    expect(body).toContain('width: 0%');
    expect(body).toContain('aria-valuetext="Unavailable"');
    expect(body).not.toContain('aria-valuenow=');
  });

  it('shows pack and per-cell voltage when the configured series count is available', () => {
    const sample = {
      packVoltageV: 75.6,
      batteryLevel: 0.64,
      charging: false,
    } as LiveSample;
    const { body } = render(GaugePower, {
      props: { sample, estimate: { ...emptyEstimate(), level: 0.5, seriesCount: 18 } },
    });

    expect(body).toContain('75.6 V');
    expect(body).toContain('4.20 V');
    expect(body).toContain('50%');
  });

  it('uses subtle text for unavailable voltage and percentage labels', () => {
    const sample = {
      packVoltageV: null,
      batteryLevel: null,
      charging: false,
    } as LiveSample;
    const { body } = render(GaugePower, {
      props: { sample, estimate: emptyEstimate() },
    });

    expect(body.match(/text-muted/g)).toHaveLength(4);
    expect(body.match(/>—<\/span>|>—<\/output>/g)).toHaveLength(1);
  });
});
