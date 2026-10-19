import { describe, expect, it, vi } from 'vitest';
import type { RefloatConfigurationValues, RefloatTune } from '$lib/backend/contract';
import {
  captureCurrentRefloatTune,
  overwriteCurrentRefloatTune,
  type RefloatTuneCaptureBackend,
} from './refloat-tunes';

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

describe('Tune overwrite', () => {
  const existing: RefloatTune = {
    tuneId: 'saved-tune',
    revision: 4,
    name: 'Street',
    description: 'Keep this description',
    identity: current.identity,
    values: { gain: { type: 'double', value: 0.5 } },
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
  };

  it('replaces only tune values using a fresh board read and the saved revision', async () => {
    const readRefloatConfiguration = vi.fn(async () => current);
    const updateRefloatTune = vi.fn(async () => ({ ...existing, revision: 5, values: { gain: current.values.gain! } }));
    const result = await overwriteCurrentRefloatTune({ readRefloatConfiguration, updateRefloatTune }, existing, [
      'gain',
    ]);
    expect(readRefloatConfiguration).toHaveBeenCalledWith('current', true);
    expect(updateRefloatTune).toHaveBeenCalledWith(existing.tuneId, 4, existing.name, existing.description, {
      gain: current.values.gain,
    });
    expect(result).toMatchObject({ current, tune: { tuneId: existing.tuneId, revision: 5 } });
  });

  it('does not overwrite after a failed board read', async () => {
    const updateRefloatTune = vi.fn();
    expect(
      await overwriteCurrentRefloatTune({ readRefloatConfiguration: async () => null, updateRefloatTune }, existing, [
        'gain',
      ])
    ).toBeNull();
    expect(updateRefloatTune).not.toHaveBeenCalled();
  });

  it.each([{ packageVersion: '1.3.0' }, { schemaFingerprint: 'b'.repeat(64) }, { wireSignature: 456 }])(
    'rejects a changed board identity: %j',
    async (identity) => {
      const updateRefloatTune = vi.fn();
      await expect(
        overwriteCurrentRefloatTune(
          {
            readRefloatConfiguration: async () => ({ ...current, identity: { ...current.identity, ...identity } }),
            updateRefloatTune,
          },
          existing,
          ['gain']
        )
      ).rejects.toThrow('different Refloat version or configuration');
      expect(updateRefloatTune).not.toHaveBeenCalled();
    }
  );

  it('rejects incomplete fields without replacing the existing tune', async () => {
    const updateRefloatTune = vi.fn();
    await expect(
      overwriteCurrentRefloatTune({ readRefloatConfiguration: async () => current, updateRefloatTune }, existing, [
        'gain',
        'missing',
      ])
    ).rejects.toThrow('complete Tune fields');
    expect(updateRefloatTune).not.toHaveBeenCalled();
  });

  it('keeps a rejected update as a failure', async () => {
    const updateRefloatTune = vi.fn(async () => null);
    expect(
      await overwriteCurrentRefloatTune(
        { readRefloatConfiguration: async () => current, updateRefloatTune },
        existing,
        ['gain']
      )
    ).toBeNull();
    expect(updateRefloatTune).toHaveBeenCalledOnce();
  });
});
