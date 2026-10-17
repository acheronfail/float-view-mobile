import { expect, it } from 'vitest';
import { MAX_SESSION_DATA_FIELDS, type SessionFieldDescriptor } from '$lib/backend/contract';
import { sessionChartGroups, sessionFieldIsChartable, sessionFieldMetadata } from './session-fields';
import { sessionSeriesGroups } from './session-series-groups';

it('loads stable and extension lines of one chart together', () => {
  const fields: SessionFieldDescriptor[] = [
    { name: 'packVoltageV', source: 'stable' },
    { name: 'motorCurrentA', source: 'stable' },
    { name: 'chargingVoltageV', source: 'extension' },
    { name: 'chargingCurrentA', source: 'extension' },
  ];
  expect(sessionSeriesGroups(fields)).toEqual([[fields[0], fields[2], fields[1], fields[3]]]);
});

it('keeps complete charts within request limits and preserves every descriptor once', () => {
  const fields: SessionFieldDescriptor[] = Object.keys(sessionFieldMetadata)
    .filter((name) => sessionFieldIsChartable(name) && name !== 'netEnergyWh')
    .map((name) => ({ name, source: 'stable' }));
  for (let index = 0; index < MAX_SESSION_DATA_FIELDS * 2; index++)
    fields.push({ name: `extension-${index}`, source: 'extension' });
  const groups = sessionSeriesGroups(fields);
  expect(groups.length).toBeGreaterThan(3);
  expect(groups.every((group) => group.length > 0 && group.length <= MAX_SESSION_DATA_FIELDS)).toBe(true);
  expect(groups.flat().sort((a, b) => a.name.localeCompare(b.name))).toEqual(
    [...fields].sort((a, b) => a.name.localeCompare(b.name))
  );
  for (const chart of sessionChartGroups(fields)) {
    const inputs = chart.fields.filter((field) => field.name !== 'netEnergyWh');
    const owners = inputs.map((field) => groups.findIndex((group) => group.some((input) => input.name === field.name)));
    expect(new Set(owners).size, chart.id).toBe(1);
  }
  expect(groups.at(-1)!.map((field) => field.name)).toEqual(['wattHoursDrawnWh', 'wattHoursChargedWh']);
});

it('handles empty fields and a single energy counter without adding derived query fields', () => {
  expect(sessionSeriesGroups([])).toEqual([]);
  const energy: SessionFieldDescriptor = { name: 'wattHoursDrawnWh', source: 'stable' };
  expect(sessionSeriesGroups([energy])).toEqual([[energy]]);
});
