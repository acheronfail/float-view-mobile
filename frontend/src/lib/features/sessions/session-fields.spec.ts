import { describe, expect, it } from 'vitest';
import {
  formatSessionFieldValue,
  sessionChartGroupMetadata,
  sessionChartGroups,
  sessionChartField,
  sessionFieldIsChartable,
  sessionFieldMetadata,
  sessionFieldValueFormat,
  sortSessionFields,
} from './session-fields';

describe('session chart field metadata', () => {
  it('defines each shared chart title once', () => {
    expect(sessionChartGroupMetadata.temperature.title).toBe('Temperature');
    expect(sessionChartGroupMetadata.current.title).toBe('Current');
    expect(sessionChartGroupMetadata.torqueControl.title).toBe('Torque control');
    expect(sessionChartGroupMetadata.fieldWeakening.title).toBe('Field weakening');
    expect(sessionChartGroupMetadata.voltage.title).toBe('Voltage');
    expect(sessionChartGroupMetadata.setpoints.title).toBe('Setpoints');
    expect(sessionChartGroupMetadata.energy.title).toBe('Energy');
  });

  it('groups torque control fields and excludes the booster contribution', () => {
    const groups = sessionChartGroups([
      { name: 'requestedBalanceCurrentA' },
      { name: 'directionalMotorCurrentA' },
      { name: 'filteredMotorCurrentA' },
      { name: 'boosterCurrentA' },
      { name: 'fieldWeakeningCurrentA' },
    ]);

    expect(groups.find((group) => group.id === 'torqueControl')?.fields.map((field) => field.name)).toEqual([
      'directionalMotorCurrentA',
      'filteredMotorCurrentA',
      'requestedBalanceCurrentA',
    ]);
    expect(groups.find((group) => group.id === 'fieldWeakening')?.fields.map((field) => field.name)).toEqual([
      'fieldWeakeningCurrentA',
    ]);
    expect(groups.flatMap((group) => group.fields.map((field) => field.name))).not.toContain('boosterCurrentA');
  });

  it('groups compatible temperature fields in metadata order', () => {
    const groups = sessionChartGroups([
      { name: 'motorTempC' },
      { name: 'bmsHighestCellTempC' },
      { name: 'controllerTempC' },
      { name: 'bmsCircuitryTempC' },
      { name: 'speedKph' },
    ]);
    const temperature = groups.find((group) => group.id === 'temperature');

    expect(temperature?.title).toBe('Temperature');
    expect(temperature?.unit).toBe('°C');
    expect(temperature?.fields.map((field) => field.name)).toEqual([
      'controllerTempC',
      'motorTempC',
      'bmsCircuitryTempC',
      'bmsHighestCellTempC',
    ]);
    expect(groups.map((group) => group.id)).toEqual(['speedKph', 'temperature']);
  });

  it('uses session metadata insertion order for chart groups and fields', () => {
    const fields = [
      { name: 'speedKph' },
      { name: 'packVoltageV' },
      { name: 'estimatedBatteryLevel' },
      { name: 'controllerTempC' },
      { name: 'motorTempC' },
    ];
    const groups = sessionChartGroups(fields);

    expect(groups.map((group) => group.id)).toEqual(['estimatedBatteryLevel', 'voltage', 'speedKph', 'temperature']);
    expect(sortSessionFields(fields).map((field) => field.name)).toEqual([
      'estimatedBatteryLevel',
      'packVoltageV',
      'speedKph',
      'controllerTempC',
      'motorTempC',
    ]);
  });

  it('charts every metadata field except those intentionally hidden', () => {
    const fields = Object.keys(sessionFieldMetadata).map((name) => ({ name }));
    const chartedNames = new Set(
      sessionChartGroups(fields).flatMap((group) => group.fields.map((field) => field.name))
    );

    expect(chartedNames).toEqual(
      new Set(Object.keys(sessionFieldMetadata).filter((name) => sessionFieldIsChartable(name)))
    );
  });

  it('falls back to a standalone chart for extension fields', () => {
    const field = sessionChartField('extensionLate');
    const groups = sessionChartGroups([{ name: 'extensionLate' }]);

    expect(field.group).toBe('extensionLate');
    expect(field.displayName).toBe('extensionLate');
    expect(groups[0]?.fields).toHaveLength(1);
  });

  it('hides Ah counters and groups consumed, recovered and net Wh', () => {
    const groups = sessionChartGroups([
      { name: 'ampHoursDrawnAh' },
      { name: 'ampHoursChargedAh' },
      { name: 'wattHoursDrawnWh' },
      { name: 'wattHoursChargedWh' },
    ]);
    expect(groups.map((group) => group.id)).toEqual(['energy']);
    expect(groups[0]!.fields.map((field) => field.displayName)).toEqual(['Consumed', 'Recovered', 'Net consumed']);
    expect(sessionChartGroups([{ name: 'wattHoursDrawnWh' }])[0]!.fields.map((field) => field.name)).toEqual([
      'wattHoursDrawnWh',
    ]);
  });

  it('hides Refloat diagnostic recording from session charts while retaining other extensions', () => {
    expect(sessionFieldIsChartable('refloat.data_recording')).toBe(false);
    expect(
      sessionChartGroups([{ name: 'refloat.data_recording' }, { name: 'extensionLate' }]).map((g) => g.id)
    ).toEqual(['extensionLate']);
  });

  it('keeps duty cycle as a ratio while formatting it as a percentage', () => {
    const field = sessionChartField('dutyCycle');
    const groups = sessionChartGroups([{ name: 'dutyCycle' }]);

    expect(field.axis).toEqual({ kind: 'fixed', min: 0, max: 1 });
    expect(field.format).toBe('percent');
    expect(groups[0]?.axis).toEqual({ kind: 'fixed', min: 0, max: 1 });
    expect(field.precision).toBe(0);
    expect(sessionFieldValueFormat('dutyCycle')).toBe('percent');
    expect(formatSessionFieldValue('dutyCycle', 0)).toBe('0%');
    expect(formatSessionFieldValue('dutyCycle', 0.5)).toBe('50%');
    expect(formatSessionFieldValue('dutyCycle', 1)).toBe('100%');
  });

  it('uses chart-specific precision before the default', () => {
    expect(sessionChartField('controllerTempC').precision).toBe(1);
    expect(formatSessionFieldValue('controllerTempC', 30.6875)).toBe('30.7');
    expect(sessionChartField('motorCurrentA').precision).toBe(2);
    expect(formatSessionFieldValue('motorCurrentA', 12.3456)).toBe('12.35');
  });
});

it('groups recorded load fractions with duty and the gauge colours', () => {
  const group = sessionChartGroups([{ name: 'batteryLoad' }, { name: 'dutyCycle' }, { name: 'motorLoad' }])[0]!;
  expect(group.title).toBe('Load');
  expect(group.axis).toEqual({ kind: 'fixed', min: 0, max: 1 });
  expect(group.fields.map((field) => field.name)).toEqual(['dutyCycle', 'motorLoad', 'batteryLoad']);
  expect(group.fields.map((field) => field.color)).toEqual([
    'var(--app-duty)',
    'var(--app-danger)',
    'var(--app-success)',
  ]);
  expect(formatSessionFieldValue('motorLoad', 0.6)).toBe('60%');
});

it('charts the recorded estimate and keeps controller percentage out of charts', () => {
  expect(sessionFieldIsChartable('batteryLevel')).toBe(false);
  expect(sessionChartGroups([{ name: 'batteryLevel' }, { name: 'estimatedBatteryLevel' }]).map((g) => g.id)).toEqual([
    'estimatedBatteryLevel',
  ]);
  expect(formatSessionFieldValue('estimatedBatteryLevel', 0.61)).toBe('61%');
});

it('shows both remote input directions on a fixed percentage axis', () => {
  const group = sessionChartGroups([{ name: 'remoteInput' }])[0]!;
  expect(group.axis).toEqual({ kind: 'fixed', min: -1, max: 1 });
  expect([-1, -0.05, 0, 1].map((value) => formatSessionFieldValue('remoteInput', value))).toEqual([
    '-100%',
    '-5%',
    '0%',
    '100%',
  ]);
});
