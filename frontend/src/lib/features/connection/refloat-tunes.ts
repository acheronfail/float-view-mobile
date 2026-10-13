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
  const current = await backend.readRefloatConfiguration('current', true);
  if (!current) return null;
  const values = Object.fromEntries(
    tuneFieldNames.flatMap((fieldName) => {
      const value = current.values[fieldName];
      return value ? [[fieldName, value] as const] : [];
    })
  );
  if (Object.keys(values).length !== tuneFieldNames.length || !tuneFieldNames.length) return null;
  const tune = await backend.createRefloatTune(name, description, current.identity, current.revision, values);
  return tune ? { current, tune } : null;
}
