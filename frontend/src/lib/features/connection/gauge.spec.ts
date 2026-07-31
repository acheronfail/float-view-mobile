import { describe, expect, it } from 'vitest';
import type { ControllerCurrentLimits } from '../../backend/contract';
import { driveLoadPercent } from './gauge';

const limits: ControllerCurrentLimits = {
	motorCurrentMaxA: 100,
	motorCurrentMinA: -50,
	batteryCurrentMaxA: 50,
	batteryCurrentMinA: -20
};

describe('drive load gauge', () => {
	it('uses raw absolute duty and filtered motor-current saturation', () => {
		expect(
			driveLoadPercent(
				{
					dutyCycle: 0.45,
					motorCurrentA: 40,
					directionalMotorCurrentA: 55,
					filteredMotorCurrentA: 60,
					batteryCurrentA: 20
				},
				limits
			)
		).toBeCloseTo(60);
	});

	it('does not apply the LED bar duty scaling', () => {
		expect(
			driveLoadPercent(
				{
					dutyCycle: -0.9,
					motorCurrentA: null,
					directionalMotorCurrentA: null,
					filteredMotorCurrentA: null,
					batteryCurrentA: null
				},
				limits
			)
		).toBeCloseTo(90);
	});

	it('selects braking and regeneration limits by current sign', () => {
		expect(
			driveLoadPercent(
				{
					dutyCycle: -0.18,
					motorCurrentA: -10,
					directionalMotorCurrentA: -25,
					filteredMotorCurrentA: -30,
					batteryCurrentA: -15
				},
				limits
			)
		).toBeCloseTo(75);
	});
});
