import { describe, expect, it } from 'vitest';
import type { ControllerCurrentLimits } from '../../backend/contract';
import { driveLoad, driveLoadPercent } from './gauge';

const limits: ControllerCurrentLimits = {
	motorCurrentMaxA: 100,
	motorCurrentMinA: -50,
	batteryCurrentMaxA: 50,
	batteryCurrentMinA: -20,
	batterySeriesCells: 18
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

	it('reports the source with the largest utilization', () => {
		expect(
			driveLoad(
				{
					dutyCycle: 0.45,
					motorCurrentA: 60,
					directionalMotorCurrentA: 60,
					filteredMotorCurrentA: 60,
					batteryCurrentA: 20
				},
				limits
			)
		).toEqual({ source: 'motor', percentage: 60 });
	});

	it('matches Refloat tie ordering: duty, then motor, then battery', () => {
		expect(
			driveLoad(
				{
					dutyCycle: 0.5,
					motorCurrentA: 50,
					directionalMotorCurrentA: 50,
					filteredMotorCurrentA: 50,
					batteryCurrentA: 25
				},
				limits
			)
		).toEqual({ source: 'duty', percentage: 50 });
	});
});
