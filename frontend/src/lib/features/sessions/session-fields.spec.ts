import { describe, expect, it } from 'vitest';
import {
  formatSessionFieldValue,
  sessionChartGroupMetadata,
  sessionChartGroups,
  sessionChartField,
  sessionFieldValueFormat,
} from './session-fields';

describe('session chart field metadata', () => {
  it('defines each shared chart title once', () => {
    expect(sessionChartGroupMetadata.temperature.title).toBe('Temperature');
    expect(sessionChartGroupMetadata.current.title).toBe('Current');
  });

  it('groups compatible temperature fields in metadata order', () => {
    const groups = sessionChartGroups([{ name: 'motorTempC' }, { name: 'controllerTempC' }, { name: 'speedKph' }]);
    const temperature = groups.find((group) => group.id === 'temperature');

    expect(temperature?.title).toBe('Temperature');
    expect(temperature?.unit).toBe('°C');
    expect(temperature?.fields.map((field) => field.name)).toEqual(['motorTempC', 'controllerTempC']);
    expect(groups.map((group) => group.id)).toEqual(['temperature', 'speedKph']);
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
