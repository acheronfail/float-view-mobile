import type { UiCheckpoint, UiRoute, UiSurface, AppState } from '$lib/backend/contract';
import { bmsMenuVisible } from '$lib/features/connection/bms';
import { flywheelOverlayVisible } from '$lib/features/connection/live';
import { routeSearchParams } from './navigation';
import { connectionHref } from './connection-navigation';

export const UI_RESTORATION = Symbol('ui-restoration');

export function uiRoute(routeId: string | null, params: Record<string, string | undefined>, url: URL): UiRoute {
  const query = routeSearchParams(url);
  switch (routeId) {
    case '/live':
      return { kind: 'live' };
    case '/bms':
      return { kind: 'bms' };
    case '/sessions':
      return { kind: 'sessions' };
    // The read-only notices page restores to its parent, without a wire-contract change.
    case '/settings/licences':
    case '/settings':
      return { kind: 'settings' };
    case '/sessions/boards/[savedDeviceId]':
      return { kind: 'boardHistory', savedDeviceId: params.savedDeviceId! };
    case '/sessions/[sessionId]':
      return {
        kind: 'session',
        sessionId: params.sessionId!,
        savedDeviceId: query.get('board'),
        returnTo: query.get('from') === 'live' ? 'live' : query.has('board') ? 'boardHistory' : 'sessions',
      };
    default:
      return { kind: 'home' };
  }
}
export function uiHref(route: UiRoute): string {
  switch (route.kind) {
    case 'home':
      return '#/';
    case 'boardHistory':
      return `#/sessions/boards/${encodeURIComponent(route.savedDeviceId)}`;
    case 'session':
      return `#/sessions/${encodeURIComponent(route.sessionId)}${route.returnTo === 'live' ? '?from=live' : route.returnTo === 'boardHistory' && route.savedDeviceId ? `?board=${encodeURIComponent(route.savedDeviceId)}` : ''}`;
    default:
      return `#/${route.kind}`;
  }
}
export function resolveUiCheckpoint(checkpoint: UiCheckpoint, state: AppState): UiCheckpoint {
  const result = structuredClone(checkpoint);
  const route = result.route;
  if (route.kind === 'boardHistory' && !state.savedDevices.some((d) => d.savedDeviceId === route.savedDeviceId)) {
    result.route = { kind: 'sessions' };
    result.surface = null;
  }
  if (
    route.kind === 'session' &&
    route.savedDeviceId &&
    !state.savedDevices.some((d) => d.savedDeviceId === route.savedDeviceId)
  ) {
    route.savedDeviceId = null;
    if (route.returnTo === 'boardHistory') route.returnTo = 'sessions';
  }
  if (route.kind === 'session' && route.returnTo === 'live' && state.activeConnection?.deviceKind !== 'board') {
    route.returnTo = 'sessions';
  }
  if (result.route.kind === 'live' || result.route.kind === 'bms') {
    const href = connectionHref(state.activeConnection);
    if (!state.activeConnection) result.route = { kind: 'home' };
    else if (href === '#/bms') result.route = { kind: 'bms' };
    else result.route = { kind: 'live' };
    if (result.live?.deviceId !== state.activeConnection?.deviceId || result.route.kind !== 'live') {
      result.live = null;
      result.surface = null;
    }
  }
  const connection = state.activeConnection;
  if (result.route.kind === 'live' && result.surface) {
    const surface = result.surface;
    const blocked =
      !connection ||
      flywheelOverlayVisible(connection) ||
      connection.state === 'selecting_controller' ||
      !!connection.connectionIssue ||
      connection.firmware?.supported === false;
    const available =
      !blocked &&
      (surface.kind === 'bmsDetails'
        ? bmsMenuVisible(connection.bms)
        : surface.kind === 'refloatConfiguration'
          ? connection.refloatConfiguration?.availability === 'ready'
          : surface.kind === 'refloatTunes'
            ? connection.refloatConfiguration?.availability === 'ready' &&
              !!connection.refloatConfiguration.packageVersion &&
              !!connection.refloatConfiguration.schemaFingerprint
            : surface.kind === 'boardConfiguration'
              ? surface.savedDeviceId === connection.savedDeviceId &&
                state.savedDevices.some((d) => d.savedDeviceId === surface.savedDeviceId)
              : true);
    if (!available) result.surface = null;
  }
  return result;
}

/** One in-flight save and one replacement; no timer or durable state. */
export class UiRestoration {
  pending = $state<UiCheckpoint | null>(null);
  owner = $state('');
  live = $state<UiCheckpoint['live']>(null);
  surface = $state<UiSurface | null>(null);
  private busy = false;
  private queued: UiCheckpoint | undefined;
  private last = '';
  private disposed = false;
  constructor(private readonly save: (checkpoint: UiCheckpoint) => Promise<void>) {}
  take(kind: UiRoute['kind']): UiCheckpoint | null {
    if (this.pending?.route.kind !== kind) return null;
    const checkpoint = this.pending;
    this.pending = null;
    return checkpoint;
  }
  present(owner: string, live: UiCheckpoint['live'], surface: UiSurface | null) {
    this.owner = owner;
    this.live = live;
    this.surface = surface;
  }
  capture(checkpoint: UiCheckpoint) {
    const encoded = JSON.stringify(checkpoint);
    if (this.disposed || encoded === this.last) return;
    this.last = encoded;
    this.queued = JSON.parse(encoded) as UiCheckpoint;
    void this.drain();
  }
  private async drain() {
    if (this.busy || this.disposed) return;
    this.busy = true;
    while (this.queued && !this.disposed) {
      const checkpoint = this.queued;
      this.queued = undefined;
      try {
        await this.save(checkpoint);
        uiDiagnostic('captured', checkpoint.route.kind, checkpoint.surface?.kind);
      } catch {
        uiDiagnostic('request_failed', checkpoint.route.kind, checkpoint.surface?.kind);
      }
    }
    this.busy = false;
  }
  dispose() {
    this.disposed = true;
    this.queued = undefined;
  }
}
export function uiDiagnostic(
  outcome: 'captured' | 'restored' | 'request_failed' | 'surface_unavailable',
  kind: UiRoute['kind'],
  surface?: UiSurface['kind']
) {
  if (import.meta.env.DEV) console.debug('[FloatView UI]', outcome, kind, surface ?? 'none');
}
