import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import GaugeBar from './WidgetBar.svelte';

describe('horizontal gauge bar', () => {
  it('renders ticks at a fixed spacing by default', () => {
    const { body } = render(GaugeBar, {
      props: {
        value: 50,
        maximum: 100,
        ariaLabel: 'Battery level',
        ariaValueText: '50 percent',
      },
    });

    expect(body).toContain('transparent 32px');
    expect(body).toContain('aria-valuenow="50"');
  });

  it('allows callers to hide ticks', () => {
    const { body } = render(GaugeBar, {
      props: {
        value: 50,
        maximum: 100,
        ariaLabel: 'Temperature',
        ariaValueText: '35 °C',
        showTicks: false,
      },
    });

    expect(body).not.toContain('background-image');
  });

  it('renders the optional label, detail, and scale from the bar component', () => {
    const { body } = render(GaugeBar, {
      props: {
        value: 50,
        maximum: 100,
        ariaLabel: 'Motor temperature',
        ariaValueText: '29.2 °C',
        label: 'Motor',
        detail: '29.2 °C',
        showLabels: true,
        tickValues: [0, 50, 100],
        format: (value) => `${value}°`,
      },
    });

    expect(body).toContain('Motor');
    expect(body).toContain('29.2 °C');
    expect(body).toContain('0°');
    expect(body).toContain('50°');
    expect(body).toContain('100°');
  });
});
