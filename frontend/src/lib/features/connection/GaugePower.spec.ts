import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { LiveSample } from '../../backend/contract';
import GaugePower from './GaugePower.svelte';

describe('battery power gauge', () => {
	it('shows pack and per-cell voltage when the configured series count is available', () => {
		const sample = {
			packVoltageV: 75.6,
			batteryLevel: 0.64,
			charging: false
		} as LiveSample;
		const { body } = render(GaugePower, {
			props: { sample, batterySeriesCells: 18 }
		});

		expect(body).toContain('75.60 V (4.20 V)');
		expect(body).toContain('64%');
	});

	it('uses subtle text for unavailable voltage and percentage labels', () => {
		const sample = {
			packVoltageV: null,
			batteryLevel: null,
			charging: false
		} as LiveSample;
		const { body } = render(GaugePower, {
			props: { sample, batterySeriesCells: null }
		});

		expect(body.match(/text-muted/g)).toHaveLength(2);
		expect(body.match(/>—<\/span>|>—<\/output>/g)).toHaveLength(2);
	});
});
