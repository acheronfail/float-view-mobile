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
    expect(sessionChartGroupMetadata.charge.title).toBe('Charge');
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
      { name: 'batteryLevel' },
      { name: 'controllerTempC' },
      { name: 'motorTempC' },
    ];
    const groups = sessionChartGroups(fields);

    expect(groups.map((group) => group.id)).toEqual(['batteryLevel', 'voltage', 'speedKph', 'temperature']);
    expect(sortSessionFields(fields).map((field) => field.name)).toEqual([
      'batteryLevel',
      'packVoltageV',
      'speedKph',
      'controllerTempC',
      'motorTempC',
    ]);
  });

  it('charts every stable field except the intentionally hidden booster contribution', () => {
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
