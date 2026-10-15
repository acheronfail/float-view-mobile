import { describe, expect, it } from 'vitest';
import { driveLoad, driveLoadComponents } from './gauge';
describe('authoritative load presentation', () => {
  it('renders backend winner without choosing a different source', () => {
    const sample = {
      load: { duty: 0.8, motor: 0.8, battery: 0.2, winner: { source: 'duty' as const, fraction: 0.8 } },
    };
    expect(driveLoad(sample)).toEqual({ source: 'duty', percentage: 80 });
    expect(driveLoadComponents(sample)).toEqual({ duty: 80, motor: 80, battery: 20 });
  });
  it('keeps missing components unavailable', () => {
    expect(driveLoad(null)).toBeNull();
    expect(
      driveLoadComponents({
        load: { duty: 0.4, motor: null, battery: null, winner: { source: 'duty', fraction: 0.4 } },
      })
    ).toEqual({ duty: 40, motor: null, battery: null });
  });
});
