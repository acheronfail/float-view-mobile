<script lang="ts">
  import { getContext, onDestroy, onMount, tick } from 'svelte';
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
  import BottomSheet from '$lib/ui/BottomSheet.svelte';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import { locationRequiresAction } from '$lib/features/connection/location-presentation';

  const viewOptions = [
    { type: 'normal', displayName: 'Normal View' },
    { type: 'detailed', displayName: 'Detailed View' },
  ] as const;
  type View = (typeof viewOptions)[number]['type'];

  // TODO: persist this state
  let view = $state<View>('normal');
  let configuration = $state<{ savedDeviceId: string; batteryOnly: boolean; trigger?: HTMLButtonElement } | null>(null);
  let bmsOpen = $state(false);
  let refloatConfigurationOpen = $state(false);
  let refloatTunesOpen = $state(false);
  let discardRefloatConfirmationOpen = $state(false);
  let disconnectConfirmationOpen = $state(false);
  let connectionIssueConfirmationOpen = $state(false);
  let firmwareUnsupportedConfirmationOpen = $state(false);
  let disconnecting = $state(false);
  let shutdownConfirmationOpen = $state(false);
  let refloatConfirmation = $state<RefloatControlConfirmation | null>(null);
  let editor = $state<BoardEditor>();
  let refloatEditor = $state<RefloatConfigurationEditor>();
  const refloatConfiguration = new RefloatConfigurationController(backendState);
  const appHeader = getContext<AppHeaderContext>(APP_HEADER_CONTEXT);
  const headerOwner = '/live';
  const connection = $derived(backendState.snapshot?.activeConnection);
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
      menu: [
        {
          label: nextView.displayName,
          onactivate: (closeMenu: () => void) => cycleView(closeMenu),
        },
        ...(connection.trackingMode === 'saved' && connection.activeSession
          ? [
              {
                label: 'Charts…',
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
          disabled: action.disabled,
          pending: action.pending,
          appearance: action.kind === 'clearFatalError' ? ('warning' as const) : ('normal' as const),
          onactivate: (closeMenu: () => void) => activateRefloatAction(action, closeMenu),
        })),
        ...(connection.refloatConfiguration
          ? [
              {
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
                disabled: connection.refloatConfiguration.availability !== 'ready',
                onactivate: (closeMenu: () => void) => {
                  closeMenu();
                  refloatTunesOpen = true;
                },
              },
            ]
          : []),
        {
          label: 'Shut down controller',
          appearance: 'destructive' as const,
          disabled: backendState.busy || controllerShutdownDisabled(connection),
          onactivate: (closeMenu: () => void) => {
            closeMenu();
            shutdownConfirmationOpen = true;
          },
        },
        ...(connection.savedDeviceId
          ? [
              {
                label: 'Configure board',
                appearance: savedDevice && !savedDevice.profileComplete ? ('warning' as const) : ('normal' as const),
                onactivate: (closeMenu: () => void) => openConfigure(closeMenu),
              },
            ]
          : []),
        ...(bmsMenuVisible(connection.bms)
          ? [
              {
                label: 'BMS',
                onactivate: (closeMenu: () => void) => {
                  closeMenu();
                  bmsOpen = true;
                },
              },
            ]
          : []),
        {
          label: 'Disconnect',
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

  async function closeBms() {
    bmsOpen = false;
    await tick();
    appHeader.focusMenu();
  }

  async function requestCloseRefloatConfiguration() {
    if (refloatEditor?.hasUnsavedChanges()) {
      discardRefloatConfirmationOpen = true;
      return;
    }
    await closeRefloatConfiguration();
  }

  async function closeRefloatConfiguration() {
    refloatConfigurationOpen = false;
    discardRefloatConfirmationOpen = false;
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
        <LiveView {charging} onconfigure={openBatteryConfigure} />
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
  <BottomSheet title="Battery management system" titleId="bms-sheet-title" closeLabel="Done" onclose={closeBms}>
    <BmsView bms={connection.bms} units={backendState.settings?.unitPreference ?? 'metric'} />
  </BottomSheet>
{/if}

{#if refloatConfigurationOpen && connection?.refloatConfiguration}
  <BottomSheet
    title="Configure Refloat"
    titleId="refloat-configuration-title"
    closeLabel="Done"
    onclose={requestCloseRefloatConfiguration}
  >
    <RefloatConfigurationEditor bind:this={refloatEditor} controller={refloatConfiguration} />
  </BottomSheet>
{/if}

{#if refloatTunesOpen && connection?.refloatConfiguration}
  <BottomSheet title="Tunes" titleId="refloat-tunes-title" closeLabel="Done" onclose={closeRefloatTunes}>
    <RefloatTunes controller={refloatConfiguration} />
  </BottomSheet>
{/if}

<ConfirmDialog
  open={discardRefloatConfirmationOpen}
  titleId="refloat-discard-title"
  title="Discard unsaved Refloat changes?"
  description="Your current draft has not been written or saved as a Tune."
  confirmLabel="Discard"
  appearance="destructive"
  onconfirm={closeRefloatConfiguration}
  oncancel={() => {
    discardRefloatConfirmationOpen = false;
  }}
/>

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
  <BottomSheet
    title={configuration.batteryOnly ? 'Battery configuration' : 'Board configuration'}
    titleId="configure-sheet-title"
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
  </BottomSheet>
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
