import { MAX_UI_CHECKPOINT_BYTES, MAX_UI_TARGET_BYTES, type UiCheckpoint } from './contract.ts';

const record = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);
const keys = (v: Record<string, unknown>, names: string[]) => Object.keys(v).every((k) => names.includes(k));
const id = (v: unknown): v is string =>
  typeof v === 'string' && v.length > 0 && new TextEncoder().encode(v).length <= MAX_UI_TARGET_BYTES;

export function validUiCheckpoint(value: unknown): value is UiCheckpoint {
  if (
    !record(value) ||
    !keys(value, ['version', 'route', 'live', 'surface']) ||
    value.version !== 1 ||
    !('live' in value) ||
    !('surface' in value) ||
    new TextEncoder().encode(JSON.stringify(value)).length > MAX_UI_CHECKPOINT_BYTES
  )
    return false;
  const { route, live, surface } = value;
  if (!record(route)) return false;
  if (route.kind === 'boardHistory') {
    if (!keys(route, ['kind', 'savedDeviceId']) || !id(route.savedDeviceId)) return false;
  } else if (route.kind === 'session') {
    if (
      !keys(route, ['kind', 'sessionId', 'savedDeviceId', 'returnTo']) ||
      !id(route.sessionId) ||
      !['live', 'sessions', 'boardHistory'].includes(String(route.returnTo)) ||
      (route.returnTo === 'boardHistory' && !id(route.savedDeviceId)) ||
      !('savedDeviceId' in route) ||
      (route.savedDeviceId != null && !id(route.savedDeviceId))
    )
      return false;
  } else if (!['home', 'sessions', 'settings', 'live', 'bms'].includes(String(route.kind)) || !keys(route, ['kind']))
    return false;
  if (
    live !== null &&
    (route.kind !== 'live' ||
      !record(live) ||
      !keys(live, ['deviceId', 'mode', 'savedDeviceId']) ||
      !id(live.deviceId) ||
      (live.mode === 'custom' && !id(live.savedDeviceId)) ||
      ('savedDeviceId' in live && !id(live.savedDeviceId)) ||
      !['normal', 'detailed', 'custom'].includes(String(live.mode)))
  )
    return false;
  if (surface === null) return true;
  if (!record(surface)) return false;
  if (surface.kind === 'boardConfiguration') {
    if (
      !keys(surface, ['kind', 'savedDeviceId', 'batteryOnly']) ||
      !id(surface.savedDeviceId) ||
      typeof surface.batteryOnly !== 'boolean'
    )
      return false;
    return (
      (route.kind === 'live' && live !== null) ||
      (route.kind === 'boardHistory' && route.savedDeviceId === surface.savedDeviceId && !surface.batteryOnly)
    );
  }
  if (!keys(surface, ['kind'])) return false;
  if (route.kind === 'boardHistory') return surface.kind === 'boardActions';
  return (
    route.kind === 'live' &&
    live !== null &&
    ['liveActions', 'liveCharts', 'faultHistory', 'bmsDetails', 'refloatConfiguration', 'refloatTunes'].includes(
      String(surface.kind)
    )
  );
}
