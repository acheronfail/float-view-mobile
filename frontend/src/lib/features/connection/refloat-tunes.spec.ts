import { describe, expect, it, vi } from 'vitest';
import type { RefloatConfigurationValues } from '$lib/backend/contract';
import { captureCurrentRefloatTune, type RefloatTuneCaptureBackend } from './refloat-tunes';

const current: RefloatConfigurationValues = {
  identity: {
    packageName: 'refloat',
    packageVersion: '1.2.3',
    schemaFingerprint: 'a'.repeat(64),
    wireSignature: 123,
  },
  revision: 7,
  source: 'current',
  values: {
    gain: { type: 'double', value: 1.25 },
    'hardware.current_max': { type: 'double', value: 100 },
  },
};

describe('Tune capture', () => {
  it.each([{ fields: [] }, { fields: ['gain', 'missing'] }])(
    'rejects incomplete Tune fields: $fields',
    async ({ fields }) => {
      const createRefloatTune = vi.fn();
      await expect(
        captureCurrentRefloatTune(
          { readRefloatConfiguration: async () => current, createRefloatTune },
          'Base',
          null,
          fields
        )
      ).rejects.toThrow('The complete Tune fields are unavailable');
      expect(createRefloatTune).not.toHaveBeenCalled();
    }
  );

  it('does not create a preset when the board read fails', async () => {
    const createRefloatTune = vi.fn();
    expect(
      await captureCurrentRefloatTune({ readRefloatConfiguration: async () => null, createRefloatTune }, 'Base', null, [
        'gain',
      ])
    ).toBeNull();
    expect(createRefloatTune).not.toHaveBeenCalled();
  });

  it('always snapshots a fresh current-board read rather than editor state', async () => {
    const readRefloatConfiguration = vi.fn(async () => current);
    const createRefloatTune: RefloatTuneCaptureBackend['createRefloatTune'] = vi.fn(async () => ({
      tuneId: 'tune',
      revision: 1,
      name: 'Street',
      description: null,
      identity: current.identity,
      values: current.values,
      createdAt: '2026-01-01T00:00:00Z',
      updatedAt: '2026-01-01T00:00:00Z',
    }));

    const result = await captureCurrentRefloatTune({ readRefloatConfiguration, createRefloatTune }, 'Street', null, [
      'gain',
    ]);

    expect(readRefloatConfiguration).toHaveBeenCalledWith('current', true);
    expect(createRefloatTune).toHaveBeenCalledWith('Street', null, current.identity, 7, {
      gain: { type: 'double', value: 1.25 },
    });
    expect(result).toMatchObject({ current, tune: { name: 'Street' } });
  });
});
