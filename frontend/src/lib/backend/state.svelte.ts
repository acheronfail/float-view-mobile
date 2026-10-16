import { validateRideMarker } from './session-data';
import type {
  AppState,
  UiCheckpoint,
  PermissionRow,
  BluetoothState,
  BoardProfile,
  SessionMetadata,
  SessionListPage,
  SessionPrunePreview,
  SessionSummary,
  Settings,
  LiveWidgetField,
  LiveWidgetSlots,
  RefloatControl,
  RefloatConfigurationIdentity,
  RefloatEditorDraft,
  RefloatConfigurationSchemaPage,
  RefloatConfigurationSummary,
  RefloatConfigurationValue,
  RefloatConfigurationValues,
  RefloatTune,
  RefloatTuneListPage,
} from '$lib/backend/contract';
import { connectBootstrap, loadBootstrap, type FrontendPlatform } from './bootstrap';
import { ApiRequestError, type ApiClient } from './client';
import { SessionDataClient } from './session-data-client';
import { SessionReviewController } from './session-controller';
import { SessionExportClient, triggerBrowserDownload, type SessionExportFormat } from './session-export-client';
import { SessionImportClient } from './session-import-client';
import type { SessionDataRequest, SessionDataResult } from './session-data';

export type BackendConnectionStatus = 'starting' | 'connected' | 'reconnecting' | 'failed';
export type PermissionOutcome = 'ready' | 'pending' | 'denied' | 'restricted';
export type TrackingMode = 'saved' | 'guest';

export function permissionOutcome(state: BluetoothState): PermissionOutcome {
  switch (state.scanPermission) {
    case 'granted':
    case 'not_required':
      return 'ready';
    case 'denied':
      return 'denied';
    case 'restricted':
      return 'restricted';
    default:
      return 'pending';
  }
}

export class BackendState {
  private pruneRefresh: { operationId: string; sessionId: string } | null = null;
  status = $state<BackendConnectionStatus>('starting');
  snapshot = $state<AppState | null>(null);
  error = $state<string | null>(null);
  scanError = $state<string | null>(null);
  scanPermissionPending = $state(false);
  permissionError = $state<string | null>(null);
  permissionBusy = $state(false);
  selectedSession = $state<SessionSummary | null>(null);
  sessionNotFound = $state(false);
  boardProfileNotFound = $state(false);
  sessionMetadata = $state<SessionMetadata | null>(null);
  selectedBoardProfile = $state<BoardProfile | null>(null);
  exportStatus = $state<string | null>(null);
  busy = $state(false);
  sessionImporting = $state(false);
  platform = $state<FrontendPlatform>('web');
  private boardProfileGeneration = 0;
  private client: ApiClient | null = null;
  private sessionDataClient: SessionDataClient | null = null;
  private sessionExportClient: SessionExportClient | null = null;
  private sessionImportClient: SessionImportClient | null = null;
  private connecting: Promise<void> | null = null;
  private unsubscribeClosed: (() => void) | null = null;
  private unsubscribeState: (() => void) | null = null;

  constructor(
    private readonly bootstrapLoader = loadBootstrap,
    private readonly bootstrapConnector = connectBootstrap
  ) {}

  get savedDevices() {
    return this.snapshot?.savedDevices ?? [];
  }

  get discoveredDevices() {
    return this.snapshot?.discoveredDevices ?? [];
  }

  get scanning() {
    return this.snapshot?.bluetooth.scanning ?? false;
  }

  get firmware() {
    return this.snapshot?.activeConnection?.firmware ?? null;
  }

  get refloatControls() {
    return this.snapshot?.activeConnection?.refloatControls ?? null;
  }

  get sessionCatalogRevision() {
    return this.snapshot?.sessionCatalogRevision ?? 0;
  }

  get settings() {
    return this.snapshot?.settings ?? null;
  }

  async connect(): Promise<void> {
    if (this.connecting) return this.connecting;
    this.connecting = this.connectNow();
    try {
      await this.connecting;
    } finally {
      this.connecting = null;
    }
  }

  private async connectNow(): Promise<void> {
    this.status = this.client ? 'reconnecting' : 'starting';
    this.unsubscribeClosed?.();
    this.unsubscribeState?.();
    this.client?.close();
    this.client = null;
    try {
      const bootstrap = await this.bootstrapLoader();
      this.client = await this.bootstrapConnector(bootstrap);
      this.sessionDataClient = new SessionDataClient(bootstrap);
      this.sessionExportClient = new SessionExportClient(bootstrap);
      this.sessionImportClient = new SessionImportClient(bootstrap);
      this.platform = bootstrap.platform ?? 'web';
      this.unsubscribeClosed = this.client.onClosed(() => {
        this.status = 'failed';
        this.permissionError = 'Disconnected. Reconnect to update access.';
      });
      this.unsubscribeState = this.client.onStateChanged((state) => this.acceptState(state));
      this.acceptState(await this.client.request('state.get', {}));
      this.status = 'connected';
      this.permissionError = null;
      this.error = null;
    } catch (error) {
      this.status = 'failed';
      this.error = message(error);
    }
  }

  async getUiCheckpoint(): Promise<UiCheckpoint | null> {
    if (!this.client?.isOpen) throw new Error('Backend is not connected');
    return this.client.request('uiState.get', {});
  }

  async setUiCheckpoint(checkpoint: UiCheckpoint | null): Promise<void> {
    if (!this.client?.isOpen) throw new Error('Backend is not connected');
    await this.client.request('uiState.set', { checkpoint });
  }

  async validateUiTarget(
    route: import('./contract').UiRoute,
    surface: import('./contract').UiSurface | null
  ): Promise<void> {
    if (!this.client?.isOpen) throw new Error('Backend is not connected');
    if (route.kind === 'session') await this.client.request('session.get', { sessionId: route.sessionId });
    if (surface?.kind === 'boardConfiguration')
      await this.client.request('boardProfile.get', { savedDeviceId: surface.savedDeviceId });
  }

  async scan(): Promise<void> {
    if (this.permissionBusy || this.snapshot?.permissions.some((row) => row.pending)) return;
    const bluetooth = this.snapshot?.bluetooth;
    if (
      bluetooth &&
      (['powered_off', 'unsupported', 'resetting'].includes(bluetooth.radio) ||
        ['denied', 'restricted'].includes(bluetooth.scanPermission))
    ) {
      this.scanError = null;
      return;
    }
    await this.run(async (client) => {
      try {
        await client.request('bluetooth.scan', { timeoutMs: 5_000 });
      } catch (error) {
        if (!(error instanceof ApiRequestError) || error.code !== 'permission_required') {
          if (error instanceof ApiRequestError && error.code === 'unavailable') {
            throw new Error(
              'Scanning is temporarily unavailable. Check Bluetooth and permissions in Settings, then try again.'
            );
          }
          throw error;
        }
        this.permissionBusy = true;
        this.scanPermissionPending = true;
        let bluetooth: BluetoothState;
        try {
          bluetooth = await client.request('bluetooth.requestPermission', {}, 35_000);
        } finally {
          this.permissionBusy = false;
          this.scanPermissionPending = false;
        }
        if (permissionOutcome(bluetooth) !== 'ready') {
          throw new Error('Bluetooth permission is required before scanning.');
        }
        // Permission can resolve before CoreBluetooth reports its powered-on state.
        // The next authoritative state lets the discovery loop resume safely.
        if (bluetooth.radio !== 'powered_on') return;
        await client.request('bluetooth.scan', { timeoutMs: 5_000 });
      }
    }, 'scan');
  }

  async connectDevice(deviceId: string, trackingMode: TrackingMode = 'saved'): Promise<boolean> {
    return this.run((client) => client.request('device.connect', { deviceId, trackingMode }).then(() => undefined));
  }

  async disconnectDevice(): Promise<void> {
    await this.run((client) => client.request('device.disconnect', {}).then(() => undefined));
  }

  async shutdownController(): Promise<boolean> {
    return this.run((client) => client.request('controller.shutdown', {}).then(() => undefined));
  }

  async setRefloatControl(control: RefloatControl, enabled: boolean): Promise<boolean> {
    return this.run((client) => client.request('refloat.control.set', { control, enabled }).then(() => undefined));
  }

  async clearRefloatFatalError(): Promise<boolean> {
    return this.run((client) => client.request('refloat.fatalError.clear', {}).then(() => undefined));
  }

  async getRefloatEditorDraft(deviceId: string): Promise<RefloatEditorDraft | null> {
    if (!this.client?.isOpen) throw new Error('Backend is not connected');
    return this.client.request('refloat.configuration.draft.get', { deviceId });
  }

  async setRefloatEditorDraft(deviceId: string, baseRevision: number, draft: RefloatEditorDraft): Promise<void> {
    if (!this.client?.isOpen) throw new Error('Backend is not connected');
    await this.client.request('refloat.configuration.draft.set', { deviceId, baseRevision, draft });
  }

  openRefloatConfiguration(): Promise<RefloatConfigurationSummary | null> {
    return this.query((client) => client.request('refloat.configuration.open', {}));
  }

  refloatConfigurationSchema(cursor: string | null = null): Promise<RefloatConfigurationSchemaPage | null> {
    return this.query((client) => client.request('refloat.configuration.schema', { cursor }));
  }

  readRefloatConfiguration(source: 'current' | 'default', refresh = true): Promise<RefloatConfigurationValues | null> {
    return this.query((client) => client.request('refloat.configuration.read', { source, refresh }));
  }

  writeRefloatConfiguration(
    identity: RefloatConfigurationIdentity,
    baseRevision: number,
    values: Record<string, RefloatConfigurationValue>
  ): Promise<RefloatConfigurationValues | null> {
    return this.query((client) =>
      client.request('refloat.configuration.write', {
        packageVersion: identity.packageVersion,
        schemaFingerprint: identity.schemaFingerprint,
        wireSignature: identity.wireSignature,
        baseRevision,
        values,
      })
    );
  }

  listRefloatTunes(
    identity: RefloatConfigurationIdentity,
    limit = 50,
    cursor: string | null = null
  ): Promise<RefloatTuneListPage | null> {
    return this.query((client) => client.request('refloat.tune.list', { identity, limit, cursor }));
  }

  getRefloatTune(tuneId: string): Promise<RefloatTune | null> {
    return this.query((client) => client.request('refloat.tune.get', { tuneId }));
  }

  createRefloatTune(
    name: string,
    description: string | null,
    identity: RefloatConfigurationIdentity,
    sourceRevision: number,
    values: Record<string, RefloatConfigurationValue>
  ): Promise<RefloatTune | null> {
    return this.query((client) =>
      client.request('refloat.tune.create', {
        name,
        description,
        identity,
        sourceRevision,
        values,
      })
    );
  }

  updateRefloatTune(
    tuneId: string,
    expectedRevision: number,
    name: string,
    description: string | null,
    values: Record<string, RefloatConfigurationValue>
  ): Promise<RefloatTune | null> {
    return this.query((client) =>
      client.request('refloat.tune.update', {
        tuneId,
        expectedRevision,
        name,
        description,
        values,
      })
    );
  }

  async deleteRefloatTune(tuneId: string, expectedRevision: number): Promise<boolean> {
    return this.run((client) =>
      client.request('refloat.tune.delete', { tuneId, expectedRevision }).then(() => undefined)
    );
  }

  applyRefloatTune(
    tuneId: string,
    expectedRevision: number,
    baseRevision: number
  ): Promise<RefloatConfigurationValues | null> {
    return this.query((client) => client.request('refloat.tune.apply', { tuneId, expectedRevision, baseRevision }));
  }

  async selectController(deviceId: string, canId: number): Promise<boolean> {
    return this.run((client) => client.request('device.selectController', { deviceId, canId }).then(() => undefined));
  }

  async updateSavedDeviceName(savedDeviceId: string, customDisplayName: string | null): Promise<boolean> {
    const device = this.savedDevices.find((value) => value.savedDeviceId === savedDeviceId);
    if (!device) return false;
    return this.run(async (client) => {
      const updated = await client.request('savedDevice.update', {
        savedDeviceId,
        expectedRevision: device.revision,
        customDisplayName,
      });
      if (!this.snapshot) return;
      this.acceptState({
        ...this.snapshot,
        savedDevices: this.snapshot.savedDevices.map((value) =>
          value.savedDeviceId === savedDeviceId ? updated : value
        ),
        activeConnection:
          this.snapshot.activeConnection?.savedDeviceId === savedDeviceId
            ? {
                ...this.snapshot.activeConnection,
                name: updated.displayName,
                displayName: updated.displayName,
              }
            : this.snapshot.activeConnection,
      });
    });
  }

  async deleteSavedDevice(savedDeviceId: string): Promise<boolean> {
    return this.run(async (client) => {
      await client.request('savedDevice.delete', { savedDeviceId });
      if (this.selectedBoardProfile?.savedDeviceId === savedDeviceId) this.closeBoardProfile();
    });
  }

  async openVoiceSettings() {
    if (!this.client?.isOpen) throw new Error('Backend is not connected');
    return this.client.request('announcements.openVoiceSettings', {}, 6_000);
  }

  async testAnnouncement(kind: 'speed' | 'load' | 'combined') {
    if (!this.client?.isOpen) throw new Error('Backend is not connected');
    return this.client.request('announcements.test', { kind }, 12_000);
  }

  async updateSessionSettings(patch: Omit<Partial<Settings>, 'revision'>): Promise<boolean> {
    const settings = this.settings;
    if (!settings) return false;
    return this.run(async (client) => {
      const updated = await client.request('settings.update', {
        expectedRevision: settings.revision,
        patch,
      });
      if (this.snapshot) this.acceptState({ ...this.snapshot, settings: updated });
      if (patch.locationRecordingEnabled === true && this.snapshot?.location.permissionAction === 'requestPermission') {
        const location = await client.request('location.requestPermission', {}, 35_000);
        if (this.snapshot) this.acceptState({ ...this.snapshot, location });
      }
    });
  }

  async saveRideMarker(sessionId: string, operationId: string) {
    if (!this.client) throw new Error('Backend unavailable');
    const result = await this.client.request('session.marker.add', { sessionId, operationId });
    validateRideMarker(result.marker);
    return result.marker;
  }

  async recoverPermission(row: PermissionRow): Promise<void> {
    if (
      !this.client ||
      this.status !== 'connected' ||
      this.permissionBusy ||
      !row.action ||
      this.snapshot?.permissions.some((p) => p.pending)
    )
      return;
    this.permissionBusy = true;
    this.permissionError = null;
    try {
      await this.client.request('permissions.recover', { permission: row.permission, action: row.action }, 35_000);
      if (row.permission === 'bluetooth') this.scanError = null;
    } catch (error) {
      this.permissionError = error instanceof Error ? error.message : 'Permission recovery failed. Try again.';
    } finally {
      try {
        this.acceptState(await this.client.request('state.get', {}));
      } catch {
        /* Connection state handles reconnect. */
      }
      this.permissionBusy = false;
    }
  }

  async requestLocationPermission(): Promise<void> {
    await this.run(async (client) => {
      const action = this.snapshot?.location.permissionAction;
      const location =
        action === 'openSettings'
          ? await client.request('location.openSettings', {}, 35_000)
          : await client.request('location.requestPermission', {}, 35_000);
      if (this.snapshot) this.acceptState({ ...this.snapshot, location });
    });
  }

  async updateLiveWidgetSlot(slot: number, field: LiveWidgetField): Promise<boolean> {
    const connection = this.snapshot?.activeConnection;
    if (!connection || !Number.isInteger(slot) || slot < 0 || slot >= 5) return false;
    if (connection.trackingMode === 'guest') {
      const slots: LiveWidgetSlots = [...connection.liveWidgetSlots];
      slots[slot] = field;
      return this.updateSessionSettings({ liveWidgetSlots: slots });
    }
    const savedDeviceId = connection.savedDeviceId;
    if (!savedDeviceId) return false;
    return this.run(async (client) => {
      for (let attempt = 0; attempt < 2; attempt++) {
        const profile = await client.request('boardProfile.get', { savedDeviceId });
        if (
          this.snapshot?.activeConnection?.savedDeviceId !== savedDeviceId ||
          this.snapshot.activeConnection.deviceId !== connection.deviceId ||
          this.snapshot.activeConnection.trackingMode !== 'saved'
        )
          throw new Error('The connected board changed. Open the picker again.');
        const slots: LiveWidgetSlots = [...(profile.values.liveWidgetSlots ?? connection.liveWidgetSlots)];
        slots[slot] = field;
        try {
          await client.request('boardProfile.update', {
            savedDeviceId,
            expectedRevision: profile.revision,
            patch: { liveWidgetSlots: slots },
          });
          return;
        } catch (error) {
          if (attempt > 0 || !(error instanceof ApiRequestError) || error.code !== 'conflict') throw error;
        }
      }
    });
  }

  async openBoardProfile(savedDeviceId: string): Promise<boolean> {
    const generation = ++this.boardProfileGeneration;
    this.selectedBoardProfile = null;
    const opened = await this.run(async (client) => {
      this.boardProfileNotFound = false;
      let profile: BoardProfile;
      try {
        profile = await client.request('boardProfile.get', { savedDeviceId });
      } catch (error) {
        if (generation === this.boardProfileGeneration)
          this.boardProfileNotFound = error instanceof ApiRequestError && error.code === 'not_found';
        throw error;
      }
      if (generation === this.boardProfileGeneration) this.selectedBoardProfile = profile;
    });
    return opened && generation === this.boardProfileGeneration;
  }

  async updateBoardProfile(savedDeviceId: string, patch: Record<string, unknown>): Promise<boolean> {
    const profile = this.selectedBoardProfile;
    const generation = this.boardProfileGeneration;
    if (!profile || profile.savedDeviceId !== savedDeviceId) return false;
    return this.run(async (client) => {
      if (generation !== this.boardProfileGeneration) return;
      let updated: BoardProfile;
      try {
        updated = await client.request('boardProfile.update', {
          savedDeviceId,
          expectedRevision: profile.revision,
          patch,
        });
      } catch (error) {
        if (!(error instanceof ApiRequestError) || error.code !== 'conflict') throw error;
        if (generation !== this.boardProfileGeneration) return;
        const current = await client.request('boardProfile.get', { savedDeviceId });
        if (generation !== this.boardProfileGeneration) return;
        updated = await client.request('boardProfile.update', {
          savedDeviceId,
          expectedRevision: current.revision,
          patch,
        });
      }
      if (generation === this.boardProfileGeneration) this.selectedBoardProfile = updated;
    });
  }

  private sessionGeneration = 0;

  async openSession(sessionId: string): Promise<boolean> {
    const generation = ++this.sessionGeneration;
    return this.run(async (client) => {
      this.sessionNotFound = false;
      let metadata: SessionMetadata;
      try {
        metadata = await client.request('session.get', { sessionId });
      } catch (error) {
        if (generation === this.sessionGeneration)
          this.sessionNotFound = error instanceof ApiRequestError && error.code === 'not_found';
        throw error;
      }
      if (generation !== this.sessionGeneration) return;
      this.sessionMetadata = metadata;
      this.selectedSession = metadata.summary;
      this.exportStatus = null;
    });
  }

  async refreshSession(sessionId: string, accept: () => boolean): Promise<void> {
    const generation = this.sessionGeneration;
    if (!this.client?.isOpen) throw new Error('Backend is not connected');
    const metadata = await this.client.request('session.get', { sessionId });
    if (generation !== this.sessionGeneration || this.selectedSession?.sessionId !== sessionId || !accept()) return;
    this.sessionMetadata = metadata;
    this.selectedSession = metadata.summary;
  }

  async sessionHref(sessionId: string): Promise<string> {
    const opened = await this.openSession(sessionId);
    if (!opened || this.selectedSession?.sessionId !== sessionId) {
      return `#/sessions/${encodeURIComponent(sessionId)}`;
    }
    const savedDeviceId = this.savedDevices.find(
      (device) => device.deviceHistoryId === this.selectedSession?.deviceHistoryId
    )?.savedDeviceId;
    const boardQuery = savedDeviceId ? `?board=${encodeURIComponent(savedDeviceId)}` : '';
    return `#/sessions/${encodeURIComponent(sessionId)}${boardQuery}`;
  }

  async listSessions(savedDeviceId: string, limit: number, cursor: string | null = null): Promise<SessionListPage> {
    if (!this.client?.isOpen) await this.connect();
    if (!this.client?.isOpen) throw new Error('Backend is not connected');
    return this.client.request('session.list', { savedDeviceId, limit, cursor });
  }

  requestSessionData(request: SessionDataRequest, signal?: AbortSignal): Promise<SessionDataResult> {
    if (!this.sessionDataClient) return Promise.reject(new Error('Session-data endpoint is not connected'));
    return this.sessionDataClient.request(request, signal);
  }

  createSessionReviewController(metadata: SessionMetadata): SessionReviewController {
    if (!this.sessionDataClient) throw new Error('Session-data endpoint is not connected');
    return new SessionReviewController(metadata, this.sessionDataClient);
  }

  async exportSession(format: SessionExportFormat): Promise<void> {
    const session = this.selectedSession;
    if (!session) return;
    await this.run(async (client) => {
      if (this.platform === 'web') {
        if (!this.sessionExportClient) throw new Error('Session export download is not connected');
        const download = await this.sessionExportClient.download(session.sessionId, format);
        triggerBrowserDownload(download);
        this.exportStatus = `${download.fileName} downloaded.`;
        return;
      }
      const result = await client.request('session.export', {
        sessionId: session.sessionId,
        format,
      });
      this.exportStatus =
        result.delivery === 'presented'
          ? `${result.fileName} is ready to save or share.`
          : `${result.fileName} is staged by the test backend.`;
    });
  }

  async importSession(file: Blob): Promise<string | null> {
    let sessionId: string | null = null;
    const imported = await this.run(async () => {
      if (!this.sessionImportClient) throw new Error('Session import is not connected');
      sessionId = (await this.sessionImportClient.upload(file)).sessionId;
    });
    return imported ? sessionId : null;
  }

  async deleteSelectedSession(): Promise<boolean> {
    const session = this.selectedSession;
    if (!session) return false;
    return this.run(async (client) => {
      await client.request('session.delete', { sessionId: session.sessionId });
      this.closeSession();
    });
  }

  async previewSessionPrune(
    scope: 'one' | 'all',
    targetResolutionSeconds: number,
    sessionId?: string
  ): Promise<SessionPrunePreview | null> {
    let preview: SessionPrunePreview | null = null;
    const completed = await this.run(async (client) => {
      preview = await client.request('session.prune.preview', {
        scope,
        ...(sessionId ? { sessionId } : {}),
        targetResolutionSeconds,
      });
    });
    return completed ? preview : null;
  }

  async cancelSessionPrune(operationId: string): Promise<void> {
    await this.run(async (client) => {
      await client.request('session.prune.cancel', { operationId });
    });
  }

  async startSessionPrune(scope: 'one' | 'all', targetResolutionSeconds: number, sessionId?: string): Promise<boolean> {
    return this.run(async (client) => {
      const operation = await client.request('session.prune.start', {
        scope,
        ...(sessionId ? { sessionId } : {}),
        targetResolutionSeconds,
      });
      if (scope === 'one' && sessionId && this.selectedSession?.sessionId === sessionId) {
        if (operation.status === 'completed') {
          const metadata = await client.request('session.get', { sessionId });
          this.sessionMetadata = metadata;
          this.selectedSession = metadata.summary;
        } else {
          this.pruneRefresh = { operationId: operation.operationId, sessionId };
          this.refreshAfterPrune();
        }
      }
    });
  }

  closeBoardProfile() {
    this.boardProfileGeneration++;
    this.selectedBoardProfile = null;
  }

  closeSession() {
    this.pruneRefresh = null;
    this.sessionGeneration++;
    this.selectedSession = null;
    this.sessionMetadata = null;
    this.exportStatus = null;
  }

  close() {
    this.pruneRefresh = null;
    this.unsubscribeClosed?.();
    this.unsubscribeState?.();
    this.unsubscribeState = null;
    this.client?.close();
    this.client = null;
    this.sessionDataClient = null;
    this.sessionExportClient = null;
    this.sessionImportClient = null;
    this.platform = 'web';
  }

  private acceptState(state: AppState) {
    if (this.snapshot && state.revision < this.snapshot.revision) return;
    this.snapshot = state;
    this.refreshAfterPrune();
  }

  private refreshAfterPrune() {
    const pending = this.pruneRefresh;
    const operation = this.snapshot?.sessionOperation;
    if (
      !pending ||
      operation?.operationId !== pending.operationId ||
      !['completed', 'cancelled', 'failed'].includes(operation.status)
    )
      return;
    this.pruneRefresh = null;
    if (this.selectedSession?.sessionId === pending.sessionId) void this.openSession(pending.sessionId);
  }

  private async run(
    action: (client: ApiClient) => Promise<void>,
    errorTarget: 'general' | 'scan' = 'general'
  ): Promise<boolean> {
    if (!this.client?.isOpen) await this.connect();
    if (!this.client?.isOpen) return false;
    this.busy = true;
    if (errorTarget === 'scan') this.scanError = null;
    else this.error = null;
    try {
      await action(this.client);
      return true;
    } catch (error) {
      if (errorTarget === 'scan') this.scanError = message(error);
      else this.error = message(error);
      return false;
    } finally {
      this.busy = false;
    }
  }

  private async query<T>(action: (client: ApiClient) => Promise<T>): Promise<T | null> {
    if (!this.client?.isOpen) await this.connect();
    if (!this.client?.isOpen) return null;
    this.busy = true;
    this.error = null;
    try {
      return await action(this.client);
    } catch (error) {
      this.error = message(error);
      return null;
    } finally {
      this.busy = false;
    }
  }
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export const backendState = new BackendState();
