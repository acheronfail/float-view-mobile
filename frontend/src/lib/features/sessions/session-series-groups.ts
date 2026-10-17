import { MAX_SESSION_DATA_FIELDS, type SessionFieldDescriptor } from '$lib/backend/contract';
import { sessionChartGroups } from './session-fields';
import { ENERGY_COUNTERS } from './session-energy';

/** Keep every chart's inputs in one bounded request so lazy loading cannot omit a line. */
export function sessionSeriesGroups(fields: readonly SessionFieldDescriptor[]): SessionFieldDescriptor[][] {
  const energy = fields.filter((field) => ENERGY_COUNTERS.some((name) => name === field.name));
  const ordinary = fields.filter((field) => !ENERGY_COUNTERS.some((name) => name === field.name));
  const byName = new Map(ordinary.map((field) => [field.name, field]));
  const groups: SessionFieldDescriptor[][] = [];
  for (const chart of sessionChartGroups(ordinary)) {
    const inputs = chart.fields.map((field) => byName.get(field.name)!);
    if (!groups.length || groups.at(-1)!.length + inputs.length > MAX_SESSION_DATA_FIELDS) groups.push([]);
    groups.at(-1)!.push(...inputs);
  }
  // Energy uses its own exact-counter reader rather than an ordinary series query.
  if (energy.length) groups.push(energy);
  return groups;
}
