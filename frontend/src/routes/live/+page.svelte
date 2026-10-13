<script lang="ts">
  import { getContext, onDestroy, onMount, tick } from 'svelte';
  import { UI_RESTORATION, type UiRestoration, uiDiagnostic } from '$lib/app/ui-restoration.svelte';
  import type { UiSurface } from '$lib/backend/contract';
  import { goto } from '$app/navigation';
  import { APP_HEADER_CONTEXT, type AppHeaderContext } from '$lib/app/header';
  import { backendState } from '$lib/backend/state.svelte';
  import type { RefloatControl } from '$lib/backend/contract';
  import BoardEditor from '$lib/features/boards/BoardEditor.svelte';
  import ControllerSelection from '$lib/features/connection/ControllerSelection.svelte';
  import DetailedView from '$lib/features/connection/DetailedView.svelte';
  import FlywheelModeOverlay from '$lib/features/connection/FlywheelModeOverlay.svelte';
  import RefloatConfigurationEditor from '$lib/features/connection/RefloatConfigurationEditor.svelte';
  import RefloatTunes from '$lib/features/connection/RefloatTunes.svelte';
  import { RefloatConfigurationController } from '$lib/features/connection/refloat-configuration.svelte';
  import { liveCharging } from '$lib/features/connection/charging';
  import { liveStatus } from '$lib/features/connection/live-status';
  import LiveCharts from '$lib/features/connection/LiveCharts.svelte';
  import LiveView from '$lib/features/connection/LiveView.svelte';
  import BmsView from '$lib/features/connection/BmsView.svelte';
  import { bmsMenuVisible } from '$lib/features/connection/bms';
  import {
    controllerShutdownConfirmation,
    controllerShutdownDisabled,
    flywheelDisableUnavailable,
    flywheelOverlayVisible,
    refloatControlConfirmation,
    refloatMenuActions,
    shouldConfirmDisconnect,
    type RefloatControlConfirmation,
  } from '$lib/features/connection/live';
  import FullScreenOverlay from '$lib/ui/FullScreenOverlay.svelte';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import { locationRequiresAction } from '$lib/features/connection/location-presentation';

  const viewOptions = [
    { type: 'normal', displayName: 'Normal View' },
    { type: 'detailed', displayName: 'Detailed View' },
  ] as const;
  type View = (typeof viewOptions)[number]['type'];

  const restoration = getContext<UiRestoration>(UI_RESTORATION);
  let presentationReady = $state(false);
  let view = $state<View>('normal');
  let configuration = $state<{ savedDeviceId: string; batteryOnly: boolean; trigger?: HTMLButtonElement } | null>(null);
  let liveChartsOpen = $state(false);
  let bmsOpen = $state(false);
  let refloatConfigurationOpen = $state(false);
  let refloatTunesOpen = $state(false);
  let disconnectConfirmationOpen = $state(false);
  let connectionIssueConfirmationOpen = $state(false);
  let firmwareUnsupportedConfirmationOpen = $state(false);
  let disconnecting = $state(false);
  let shutdownConfirmationOpen = $state(false);
  let refloatConfirmation = $state<RefloatControlConfirmation | null>(null);
  let editor = $state<BoardEditor>();
  const appHeader = getContext<AppHeaderContext>(APP_HEADER_CONTEXT);
  const headerOwner = '/live';
  const connection = $derived(backendState.snapshot?.activeConnection);
  const refloatDeviceId = $derived(connection?.deviceId ?? '');
  const refloatConfiguration = $derived(new RefloatConfigurationController(backendState, refloatDeviceId));
  onMount(() => {
    const checkpoint = restoration.take('live');
    if (checkpoint?.live && checkpoint.live.deviceId === connection?.deviceId) {
      view = checkpoint.live.mode;
      const surface = checkpoint.surface;
      const blocked =
        flywheelOverlayVisible(connection) ||
        connection?.state === 'selecting_controller' ||
        !!connection?.connectionIssue ||
        connection?.firmware?.supported === false;
      if (surface && !blocked) {
        switch (surface.kind) {
          case 'liveCharts':
            liveChartsOpen = true;
            break;
          case 'bmsDetails':
            bmsOpen = !!connection && bmsMenuVisible(connection.bms);
            break;
          case 'refloatConfiguration':
            refloatConfigurationOpen = connection?.refloatConfiguration?.availability === 'ready';
            break;
          case 'refloatTunes':
            refloatTunesOpen =
              connection?.refloatConfiguration?.availability === 'ready' &&
              !!connection.refloatConfiguration.packageVersion &&
              !!connection.refloatConfiguration.schemaFingerprint;
            break;
          case 'boardConfiguration':
            if (
              surface.savedDeviceId === connection?.savedDeviceId &&
              backendState.savedDevices.some((d) => d.savedDeviceId === surface.savedDeviceId)
            )
              configuration = { savedDeviceId: surface.savedDeviceId, batteryOnly: surface.batteryOnly };
            break;
        }
      } else if (surface) uiDiagnostic('surface_unavailable', 'live', surface.kind);
    }
    presentationReady = true;
  });
  $effect(() => {
    if (!presentationReady) return;
    let surface: UiSurface | null = configuration
      ? {
          kind: 'boardConfiguration',
          savedDeviceId: configuration.savedDeviceId,
          batteryOnly: configuration.batteryOnly,
        }
      : liveChartsOpen
        ? { kind: 'liveCharts' }
        : bmsOpen
          ? { kind: 'bmsDetails' }
          : refloatConfigurationOpen
            ? { kind: 'refloatConfiguration' }
            : refloatTunesOpen
              ? { kind: 'refloatTunes' }
              : null;
    if (flywheelOverlayVisible(connection)) surface = null;
    restoration.present('#/live', connection ? { deviceId: connection.deviceId, mode: view } : null, surface);
  });
  const sample = $derived(connection?.latestSample);
  const live = $derived(connection?.state === 'ready');
  const savedDevice = $derived(
    backendState.savedDevices.find((device) => device.savedDeviceId === connection?.savedDeviceId)
  );
  const nextView = $derived.by(() => {
    const currentIndex = viewOptions.findIndex((option) => option.type === view);
    return viewOptions[(currentIndex + 1) % viewOptions.length] ?? viewOptions[0];
  });

  const location = $derived(backendState.snapshot?.location);
  const locationNeedsAction = $derived(connection?.trackingMode === 'saved' && locationRequiresAction(location));
  const connectionIssueKey = $derived(
    connection?.connectionIssue ? `${connection.deviceId}:${connection.connectionIssue.code}` : null
  );
  const unsupportedFirmwareKey = $derived(
    connection?.firmware?.supported === false
      ? `${connection.deviceId}:${connection.firmware.vescVersion}:${connection.firmware.packageVersion}`
      : null
  );

  let lastConnectionIssueKey = $state<string | null>(null);
  let lastUnsupportedFirmwareKey = $state<string | null>(null);

  $effect(() => {
    const issueKey = connectionIssueKey;
    if (!issueKey) {
      connectionIssueConfirmationOpen = false;
      lastConnectionIssueKey = null;
    } else if (issueKey !== lastConnectionIssueKey) {
      connectionIssueConfirmationOpen = true;
      lastConnectionIssueKey = issueKey;
    }
  });

  $effect(() => {
    refloatConfiguration.reconcile(connection?.refloatConfiguration ?? null);
  });

  $effect(() => {
    const firmwareKey = unsupportedFirmwareKey;
    if (!firmwareKey) {
      firmwareUnsupportedConfirmationOpen = false;
      lastUnsupportedFirmwareKey = null;
    } else if (firmwareKey !== lastUnsupportedFirmwareKey) {
      firmwareUnsupportedConfirmationOpen = true;
      lastUnsupportedFirmwareKey = firmwareKey;
    }
  });

  let nowMs = $state(Date.now());
  onMount(() => {
    const timer = setInterval(() => (nowMs = Date.now()), 1_000);
    return () => clearInterval(timer);
  });
  const charging = $derived(liveCharging(connection, Math.max(nowMs, Date.now())));
  const deviceState = $derived(liveStatus(connection?.state, sample, charging));

  $effect(() => {
    if (!connection) {
      appHeader.set(headerOwner, { title: 'Live board', subtitle: 'Recovering connection…' });
      return;
    }

    appHeader.set(headerOwner, {
      title: connection.displayName,
      bottomBorder: view === 'detailed',
      subtitle: deviceState.display,
      subtitleClass: deviceState.safetyStateActive
        ? 'text-sm text-danger'
        : live
          ? 'text-sm capitalize text-success'
          : 'text-sm capitalize text-warning',
      menuLabel: 'Board actions',
      menuPresentation: 'overlay',
      menuUnavailable: flywheelOverlayVisible(connection),
      menu: [
        {
          label: nextView.displayName,
          icon: nextView.type === 'detailed' ? 'list' : 'layout-dashboard',
          group: 'View',
          onactivate: (closeMenu: () => void) => cycleView(closeMenu),
        },
        {
          label: 'Live charts…',
          icon: 'chart-no-axes-combined',
          group: 'View',
          onactivate: (closeMenu: () => void) => {
            closeMenu();
            liveChartsOpen = true;
          },
        },
        ...(connection.trackingMode === 'saved' && connection.activeSession
          ? [
              {
                label: 'Session…',
                icon: 'chart-no-axes-combined' as const,
                group: 'View',
                onactivate: (closeMenu: () => void) => {
                  closeMenu();
                  if (connection?.activeSession)
                    void goto(`#/sessions/${encodeURIComponent(connection.activeSession.sessionId)}`);
                },
              },
            ]
          : []),
        ...refloatMenuActions(connection).map((action) => ({
          label: action.label,
          icon:
            action.kind === 'clearFatalError'
              ? ('triangle-alert' as const)
              : action.control === 'flywheel'
                ? ('rotate-cw' as const)
                : ('power' as const),
          group: 'Refloat',
          disabled: action.disabled,
          pending: action.pending,
          appearance: action.kind === 'clearFatalError' ? ('warning' as const) : ('normal' as const),
          onactivate: (closeMenu: () => void) => activateRefloatAction(action, closeMenu),
        })),
        ...(connection.refloatConfiguration
          ? [
              {
                group: 'Refloat',
                icon: 'sliders-horizontal' as const,
                label:
                  connection.refloatConfiguration.availability === 'loading'
                    ? 'Configure Refloat (loading…)'
                    : 'Configure Refloat',
                disabled: connection.refloatConfiguration.availability !== 'ready',
                onactivate: (closeMenu: () => void) => {
                  closeMenu();
                  refloatConfigurationOpen = true;
                },
              },
            ]
          : []),
        ...(connection.refloatConfiguration?.packageVersion && connection.refloatConfiguration.schemaFingerprint
          ? [
              {
                label: 'Tunes…',
                icon: 'sliders-vertical' as const,
                group: 'Refloat',
                disabled: connection.refloatConfiguration.availability !== 'ready',
                onactivate: (closeMenu: () => void) => {
                  closeMenu();
                  refloatTunesOpen = true;
                },
              },
            ]
          : []),
        ...(connection.savedDeviceId
          ? [
              {
                label: 'Configure board',
                icon: 'settings' as const,
                group: 'Board',
                appearance: savedDevice && !savedDevice.profileComplete ? ('warning' as const) : ('normal' as const),
                onactivate: (closeMenu: () => void) => openConfigure(closeMenu),
              },
            ]
          : []),
        ...(bmsMenuVisible(connection.bms)
          ? [
              {
                label: 'BMS',
                icon: 'battery' as const,
                group: 'Board',
                onactivate: (closeMenu: () => void) => {
                  closeMenu();
                  bmsOpen = true;
                },
              },
            ]
          : []),
        {
          label: 'Shut down controller',
          icon: 'power',
          group: 'Connection',
          appearance: 'destructive' as const,
          disabled: backendState.busy || controllerShutdownDisabled(connection),
          onactivate: (closeMenu: () => void) => {
            closeMenu();
            shutdownConfirmationOpen = true;
          },
        },
        {
          label: 'Disconnect',
          icon: 'bluetooth-off',
          group: 'Connection',
          appearance: 'destructive' as const,
          disabled: backendState.busy,
          onactivate: (closeMenu: () => void) => requestDisconnect(closeMenu),
        },
      ],
    });
  });
  onDestroy(() => appHeader.clear(headerOwner));

  function activateLocation() {
    if (location?.status === 'disabled') {
      void backendState.updateSessionSettings({ locationRecordingEnabled: true });
    } else {
      void backendState.requestLocationPermission();
    }
  }

  function cycleView(closeMenu: () => void) {
    closeMenu();
    view = nextView.type;
  }

  function requestDisconnect(closeMenu: () => void) {
    closeMenu();
    if (shouldConfirmDisconnect(sample?.rideState)) {
      disconnectConfirmationOpen = true;
      return;
    }
    void disconnect();
  }

  function activateRefloatAction(action: ReturnType<typeof refloatMenuActions>[number], closeMenu: () => void) {
    closeMenu();
    if (action.kind === 'clearFatalError') {
      void performFatalClear();
      return;
    }
    refloatConfirmation = refloatControlConfirmation(action.control, action.enabled);
    if (refloatConfirmation) return;
    void performRefloatControl(action.control, action.enabled);
  }

  async function performRefloatControl(control?: RefloatControl, enabled?: boolean) {
    const confirmed = refloatConfirmation;
    const selectedControl = control ?? confirmed?.control;
    const selectedEnabled = enabled ?? confirmed?.enabled;
    if (!selectedControl || selectedEnabled === undefined) return;
    await backendState.setRefloatControl(selectedControl, selectedEnabled);
    refloatConfirmation = null;
    await tick();
    appHeader.focusMenu();
  }

  async function performFatalClear() {
    await backendState.clearRefloatFatalError();
    await tick();
    appHeader.focusMenu();
  }

  async function disableFlywheelMode() {
    if (!connection || flywheelDisableUnavailable(connection)) return;
    await backendState.setRefloatControl('flywheel', false);
  }

  async function cancelRefloatConfirmation() {
    refloatConfirmation = null;
    await tick();
    appHeader.focusMenu();
  }

  async function performControllerShutdown() {
    if (!connection || controllerShutdownDisabled(connection)) return;
    await backendState.shutdownController();
    shutdownConfirmationOpen = false;
    await tick();
    if (backendState.snapshot?.activeConnection) appHeader.focusMenu();
  }

  async function cancelControllerShutdown() {
    shutdownConfirmationOpen = false;
    await tick();
    appHeader.focusMenu();
  }

  async function disconnect() {
    if (disconnecting) return;
    disconnecting = true;
    await backendState.disconnectDevice();
    disconnecting = false;
    disconnectConfirmationOpen = false;
    if (!backendState.snapshot?.activeConnection) await goto('#/', { replaceState: true });
  }

  async function cancelDisconnect() {
    disconnectConfirmationOpen = false;
    await tick();
    appHeader.focusMenu();
  }

  async function retryConnection() {
    const deviceId = connection?.deviceId;
    connectionIssueConfirmationOpen = false;
    if (deviceId) await backendState.connectDevice(deviceId, connection?.trackingMode ?? 'saved');
  }

  function continueWithUnsupportedFirmware() {
    firmwareUnsupportedConfirmationOpen = false;
  }

  async function cancelConnectionIssue() {
    connectionIssueConfirmationOpen = false;
    await disconnect();
  }

  async function cancelUnsupportedFirmware() {
    firmwareUnsupportedConfirmationOpen = false;
    await disconnect();
  }

  function openConfigure(closeMenu?: () => void) {
    closeMenu?.();
    if (connection?.savedDeviceId) configuration = { savedDeviceId: connection.savedDeviceId, batteryOnly: false };
  }

  function openBatteryConfigure(trigger: HTMLButtonElement) {
    if (connection?.savedDeviceId && !configuration) {
      configuration = { savedDeviceId: connection.savedDeviceId, batteryOnly: true, trigger };
    }
  }

  $effect(() => {
    if (configuration && (connection?.savedDeviceId !== configuration.savedDeviceId || !savedDevice)) {
      configuration = null;
    }
  });

  async function closeConfigure() {
    const closingConfiguration = configuration;
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    await tick();
    if ((await editor?.flush()) === false) return;
    if (configuration !== closingConfiguration) return;
    configuration = null;
    await tick();
    if (closingConfiguration?.trigger?.isConnected) closingConfiguration.trigger.focus();
    else appHeader.focusMenu();
  }

  async function closeLiveCharts(keyboard: boolean) {
    liveChartsOpen = false;
    await tick();
    if (keyboard) appHeader.focusMenu();
  }

  async function closeBms() {
    bmsOpen = false;
    await tick();
    appHeader.focusMenu();
  }

  async function closeRefloatConfiguration() {
    if (!(await refloatConfiguration.flush())) return;
    refloatConfigurationOpen = false;
    await tick();
    appHeader.focusMenu();
  }

  async function closeRefloatTunes() {
    refloatTunesOpen = false;
    await tick();
    appHeader.focusMenu();
  }
</script>

<section aria-labelledby="app-title" class="flex h-full min-h-0 flex-1 flex-col gap-2">
  {#if connection}
    {#if locationNeedsAction && location}
      <div
        class="flex items-center justify-between gap-3 rounded-lg border border-warning/40 bg-warning/10 px-3 py-2 text-sm"
      >
        <span>
          {location.status === 'disabled'
            ? 'Record this ride location?'
            : (location.reason ?? 'Location recording is unavailable.')}
        </span>
        <button
          type="button"
          class="touch-manipulation rounded border border-warning px-2 py-1 text-warning"
          onpointerdown={(event) => activateFromPointer(event, activateLocation)}
          onclick={(event) => activateFromKeyboardOrAssistiveClick(event, activateLocation)}
        >
          {location.status === 'disabled'
            ? 'Enable'
            : location.permissionAction === 'openSettings'
              ? 'Settings'
              : 'Allow'}
        </button>
      </div>
    {/if}

    {#if view === 'normal'}
      <div class="h-full min-h-0 flex-1 overflow-hidden">
        <LiveView
          {charging}
          {nowMs}
          onconfigure={openBatteryConfigure}
          onbms={bmsMenuVisible(connection.bms)
            ? () => {
                bmsOpen = true;
              }
            : undefined}
        />
      </div>
    {:else}
      <div class="min-h-0 flex-1 overflow-y-auto overscroll-y-auto" style="--section-header-top: 0px">
        <DetailedView {charging} onconfigure={openBatteryConfigure} />
      </div>
    {/if}
  {:else}
    <p class="text-muted">Recovering active connection…</p>
  {/if}
  {#if backendState.error}<p role="alert" class="shrink-0 text-warning">{backendState.error}</p>{/if}
</section>

{#if connection}
  <ConfirmDialog
    open={connectionIssueConfirmationOpen}
    titleId="connection-issue-confirmation-title"
    title="Connection issue"
    description={connection?.connectionIssue?.message ?? 'An unknown connection issue occurred.'}
    confirmLabel="Retry"
    appearance="warning"
    onconfirm={retryConnection}
    oncancel={cancelConnectionIssue}
  />

  <ConfirmDialog
    open={firmwareUnsupportedConfirmationOpen}
    titleId="firmware-unsupported-confirmation-title"
    title="Firmware unsupported"
    description="Firmware {connection.firmware?.vescVersion} / {connection.firmware
      ?.packageVersion} is not supported. Live values may be incomplete."
    confirmLabel="Continue"
    appearance="warning"
    onconfirm={continueWithUnsupportedFirmware}
    oncancel={cancelUnsupportedFirmware}
  />
{/if}

{#if bmsOpen && connection}
  <FullScreenOverlay title="Battery management system" titleId="bms-overlay-title" closeLabel="Done" onclose={closeBms}>
    <BmsView bms={connection.bms} units={backendState.settings?.unitPreference ?? 'metric'} />
  </FullScreenOverlay>
{/if}

{#if refloatConfigurationOpen && connection?.refloatConfiguration}
  <FullScreenOverlay
    title="Configure Refloat"
    titleId="refloat-configuration-title"
    closeLabel="Done"
    fixedContent
    headerShadow
    onclose={closeRefloatConfiguration}
  >
    <RefloatConfigurationEditor controller={refloatConfiguration} />
  </FullScreenOverlay>
{/if}

{#if refloatTunesOpen && connection?.refloatConfiguration}
  <FullScreenOverlay title="Tunes" titleId="refloat-tunes-title" closeLabel="Done" onclose={closeRefloatTunes}>
    <RefloatTunes controller={refloatConfiguration} />
  </FullScreenOverlay>
{/if}

<ConfirmDialog
  open={disconnectConfirmationOpen}
  titleId="disconnect-confirmation-title"
  title="Disconnect board?"
  description="The board is not idle. This will stop the active session and disconnect the board."
  confirmLabel="Disconnect"
  appearance="destructive"
  busy={disconnecting}
  onconfirm={disconnect}
  oncancel={cancelDisconnect}
/>

<ConfirmDialog
  open={refloatConfirmation !== null}
  titleId="refloat-control-confirmation-title"
  title={refloatConfirmation?.title ?? 'Confirm Refloat control'}
  description={refloatConfirmation?.description ?? ''}
  confirmLabel={refloatConfirmation?.confirmLabel ?? 'Confirm'}
  appearance="warning"
  busy={backendState.busy}
  onconfirm={() => performRefloatControl()}
  oncancel={cancelRefloatConfirmation}
/>

<ConfirmDialog
  open={shutdownConfirmationOpen}
  titleId="controller-shutdown-confirmation-title"
  title={controllerShutdownConfirmation.title}
  description={controllerShutdownConfirmation.description}
  confirmLabel={controllerShutdownConfirmation.confirmLabel}
  appearance="destructive"
  busy={backendState.busy || connection?.controllerActions.pendingAction === 'shutdown'}
  onconfirm={performControllerShutdown}
  oncancel={cancelControllerShutdown}
/>

{#if configuration}
  <FullScreenOverlay
    title={configuration.batteryOnly ? 'Battery configuration' : 'Board configuration'}
    titleId="configure-overlay-title"
    onclose={closeConfigure}
  >
    {#key configuration.savedDeviceId}
      <BoardEditor
        bind:this={editor}
        savedDeviceId={configuration.savedDeviceId}
        batteryOnly={configuration.batteryOnly}
        embedded
        onclose={closeConfigure}
      />
    {/key}
  </FullScreenOverlay>
{/if}

{#if connection?.state === 'selecting_controller'}
  <ControllerSelection {connection} ondisconnect={disconnect} />
{/if}

{#if flywheelOverlayVisible(connection)}
  <FlywheelModeOverlay
    busy={backendState.busy || connection?.refloatControls?.pendingAction === 'flywheel'}
    disabled={flywheelDisableUnavailable(connection)}
    ondisable={disableFlywheelMode}
  />
{/if}

{#if liveChartsOpen && !flywheelOverlayVisible(connection)}
  <LiveCharts onclose={closeLiveCharts} />
{/if}
