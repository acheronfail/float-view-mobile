import { describe, expect, it } from 'vitest';
import { firstVisibleChartField, toggleChartField } from './chart-visibility';

describe('chart visibility', () => {
  const fields = [{ name: 'motorTempC' }, { name: 'controllerTempC' }];

  it('toggles one field without changing other visibility', () => {
    const hidden = toggleChartField(new Set<string>(), 'motorTempC');
    expect([...hidden]).toEqual(['motorTempC']);
    expect(firstVisibleChartField(fields, hidden)).toBe('controllerTempC');
    expect([...toggleChartField(hidden, 'motorTempC')]).toEqual([]);
  });

  it('supports an all-hidden group and restoring any line', () => {
    const hidden = new Set(['motorTempC', 'controllerTempC']);
    expect(firstVisibleChartField(fields, hidden)).toBeNull();
    const restored = toggleChartField(hidden, 'controllerTempC');
    expect(firstVisibleChartField(fields, restored)).toBe('controllerTempC');
  });
});
