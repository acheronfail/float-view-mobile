<script lang="ts">
  import { getContext, onDestroy, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { APP_HEADER_CONTEXT, type AppHeaderContext } from '$lib/app/header';
  import { backendState } from '$lib/backend/state.svelte';
  import BoardEditor from '$lib/features/boards/BoardEditor.svelte';
  import ControllerSelection from '$lib/features/connection/ControllerSelection.svelte';
  import DetailedView from '$lib/features/connection/DetailedView.svelte';
  import LiveView from '$lib/features/connection/LiveView.svelte';
  import { shouldConfirmDisconnect } from '$lib/features/connection/live';
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
  let configureOpen = $state(false);
  let disconnectConfirmationOpen = $state(false);
  let connectionIssueConfirmationOpen = $state(false);
  let firmwareUnsupportedConfirmationOpen = $state(false);
  let disconnecting = $state(false);
  let editor = $state<BoardEditor>();
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
  const locationNeedsAction = $derived(locationRequiresAction(location));
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
    const firmwareKey = unsupportedFirmwareKey;
    if (!firmwareKey) {
      firmwareUnsupportedConfirmationOpen = false;
      lastUnsupportedFirmwareKey = null;
    } else if (firmwareKey !== lastUnsupportedFirmwareKey) {
      firmwareUnsupportedConfirmationOpen = true;
      lastUnsupportedFirmwareKey = firmwareKey;
    }
  });

  const deviceState = $derived.by(() => {
    if (connection?.state !== 'ready')
      return {
        display: connection?.state.replaceAll('_', ' '),
        safetyStateActive: false,
      };

    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    const states = new Set<string>();
    if (sample) {
      if (sample.wheelslip) states.add('Wheelslip');
      if (sample.stopCondition && sample.stopCondition !== 'none') states.add(sample.stopCondition);
      if (sample.setpointAdjustment && sample.setpointAdjustment !== 'none') states.add(sample.setpointAdjustment);
      if (sample.alertReason && sample.alertReason !== 'none') states.add(sample.alertReason);
      if (sample.firmwareFault) states.add('Firmware fault');
    }

    if (states.size === 2 && states.has('error') && isSafeState(states)) {
      states.delete('error');
      return {
        display: states.keys().next().value,
        safetyStateActive: false,
      };
    }

    return {
      display: [...states.keys()].join(' • ') || 'Connected',
      safetyStateActive: states.size > 0,
    };
  });

  $effect(() => {
    if (!connection) {
      appHeader.set(headerOwner, { title: 'Live board', subtitle: 'Recovering connection…' });
      return;
    }

    appHeader.set(headerOwner, {
      title: connection.displayName,
      subtitle: deviceState.display,
      subtitleClass: deviceState.safetyStateActive
        ? 'text-sm capitalize text-danger'
        : live
          ? 'text-sm capitalize text-success'
          : 'text-sm capitalize text-warning',
      menuLabel: 'Board actions',
      menu: [
        {
          label: nextView.displayName,
          onactivate: (closeMenu: () => void) => cycleView(closeMenu),
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

  $effect(() => {
    if (backendState.status === 'connected' && !connection) {
      void goto('#/', { replaceState: true });
    }
  });

  function activateLocation() {
    if (location?.status === 'disabled') {
      void backendState.updateSessionSettings({ locationRecordingEnabled: true });
    } else {
      void backendState.requestLocationPermission();
    }
  }

  function isSafeState(states: Set<string>) {
    return states.has('switchHalf') || states.has('reverseStop') || states.has('quickStop');
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
    if (deviceId) await backendState.connectDevice(deviceId);
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
    configureOpen = true;
  }

  async function closeConfigure() {
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    await tick();
    await editor?.flush();
    configureOpen = false;
    await tick();
    appHeader.focusMenu();
  }
</script>

<section
  aria-labelledby="app-title"
  class="flex flex-col gap-2"
  class:flex-1={view === 'normal'}
  class:min-h-0={view === 'normal'}
  class:flex-none={view === 'detailed'}
>
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
      <div class="min-h-0 flex-1 overflow-hidden">
        <LiveView />
      </div>
    {:else}
      <DetailedView />
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

{#if configureOpen && connection?.savedDeviceId}
  <BottomSheet title="Board configuration" titleId="configure-sheet-title" onclose={closeConfigure}>
    <BoardEditor bind:this={editor} savedDeviceId={connection.savedDeviceId} embedded onclose={closeConfigure} />
  </BottomSheet>
{/if}

{#if connection?.state === 'selecting_controller'}
  <ControllerSelection {connection} ondisconnect={disconnect} />
{/if}
