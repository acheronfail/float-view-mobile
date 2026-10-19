import { sameRefloatIdentity } from './refloat-configuration.svelte';
import type {
  RefloatConfigurationIdentity,
  RefloatConfigurationValue,
  RefloatConfigurationValues,
  RefloatTune,
} from '$lib/backend/contract';

export interface RefloatTuneCaptureBackend {
  readRefloatConfiguration(
    source: 'current' | 'default',
    refresh?: boolean
  ): Promise<RefloatConfigurationValues | null>;
  createRefloatTune(
    name: string,
    description: string | null,
    identity: RefloatConfigurationIdentity,
    sourceRevision: number,
    values: Record<string, RefloatConfigurationValue>
  ): Promise<RefloatTune | null>;
}

export async function captureCurrentRefloatTune(
  backend: RefloatTuneCaptureBackend,
  name: string,
  description: string | null,
  tuneFieldNames: readonly string[]
): Promise<{ current: RefloatConfigurationValues; tune: RefloatTune } | null> {
  const captured = await readCurrentTune(backend, tuneFieldNames);
  if (!captured) return null;
  const { current, values } = captured;
  const tune = await backend.createRefloatTune(name, description, current.identity, current.revision, values);
  return tune ? { current, tune } : null;
}

export interface RefloatTuneOverwriteBackend extends Pick<RefloatTuneCaptureBackend, 'readRefloatConfiguration'> {
  updateRefloatTune(
    tuneId: string,
    expectedRevision: number,
    name: string,
    description: string | null,
    values: Record<string, RefloatConfigurationValue>
  ): Promise<RefloatTune | null>;
}

export async function overwriteCurrentRefloatTune(
  backend: RefloatTuneOverwriteBackend,
  existing: RefloatTune,
  tuneFieldNames: readonly string[]
): Promise<{ current: RefloatConfigurationValues; tune: RefloatTune } | null> {
  const captured = await readCurrentTune(backend, tuneFieldNames);
  if (!captured) return null;
  if (!sameRefloatIdentity(existing.identity, captured.current.identity))
    throw new Error('This tune uses a different Refloat version or configuration.');
  const tune = await backend.updateRefloatTune(
    existing.tuneId,
    existing.revision,
    existing.name,
    existing.description,
    captured.values
  );
  return tune ? { current: captured.current, tune } : null;
}

async function readCurrentTune(
  backend: Pick<RefloatTuneCaptureBackend, 'readRefloatConfiguration'>,
  tuneFieldNames: readonly string[]
) {
  const current = await backend.readRefloatConfiguration('current', true);
  if (!current) return null;
  const values = Object.fromEntries(
    tuneFieldNames.flatMap((fieldName) => {
      const value = current.values[fieldName];
      return value ? [[fieldName, value] as const] : [];
    })
  );
  if (Object.keys(values).length !== tuneFieldNames.length || !tuneFieldNames.length) {
    throw new Error('The complete Tune fields are unavailable. Close this dialog and retry loading Tunes.');
  }
  return { current, values };
}
