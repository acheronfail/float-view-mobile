<script lang="ts">
  import { goto } from '$app/navigation';
  import { tick } from 'svelte';
  import { backendState } from '$lib/backend/state.svelte';
  import BoardEditor from '$lib/features/boards/BoardEditor.svelte';
  import ControllerSelection from '$lib/features/connection/ControllerSelection.svelte';
  import DetailedView from '$lib/features/connection/DetailedView.svelte';
  import LiveView from '$lib/features/connection/LiveView.svelte';
  import { shouldConfirmDisconnect } from '$lib/features/connection/live';
  import BottomSheet from '$lib/ui/BottomSheet.svelte';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import MenuItem from '$lib/ui/MenuItem.svelte';
  import StickyHeader from '$lib/ui/StickyHeader.svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';

  const viewOptions = [
    { type: 'normal', displayName: 'Normal View' },
    { type: 'detailed', displayName: 'Detailed View' },
  ] as const;
  type View = (typeof viewOptions)[number]['type'];

  // TODO: persist this state
  let view = $state<View>('normal');
  let configureOpen = $state(false);
  let disconnectConfirmationOpen = $state(false);
  let disconnecting = $state(false);
  let menuButton = $state<HTMLButtonElement>();
  let editor = $state<BoardEditor>();
  let liveHeaderHeight = $state(0);
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
  const locationNeedsAction = $derived(Boolean(location && location.status !== 'authorized'));

  // TODO: translate this into a display name we can show in the UI
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
    menuButton?.focus();
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
    menuButton?.focus();
  }
</script>

<section
  aria-labelledby="live-heading"
  class="flex flex-col gap-2"
  class:flex-1={view === 'normal'}
  class:min-h-0={view === 'normal'}
  class:flex-none={view === 'detailed'}
  style={`--section-header-top: calc(max(1.5rem, env(safe-area-inset-top, 0px)) + ${liveHeaderHeight}px)`}
>
  {#if connection}
    <StickyHeader
      bind:button={menuButton}
      bind:headerHeight={liveHeaderHeight}
      headingId="live-heading"
      title={connection.displayName}
      subtitle={deviceState.display}
      subtitleClass={deviceState.safetyStateActive
        ? 'mt-1 text-sm capitalize text-danger'
        : live
          ? 'mt-1 text-sm capitalize text-success'
          : 'mt-1 text-sm capitalize text-warning'}
      menuLabel="Board actions"
    >
      {#snippet menu(closeMenu)}
        <MenuItem label={nextView.displayName} onactivate={() => cycleView(closeMenu)} />
        {#if connection.savedDeviceId}
          <MenuItem
            appearance={savedDevice && !savedDevice.profileComplete ? 'warning' : 'normal'}
            label="Configure board"
            onactivate={() => openConfigure(closeMenu)}
          />
        {/if}
        <MenuItem
          label="Disconnect"
          appearance="destructive"
          disabled={backendState.busy}
          onactivate={() => requestDisconnect(closeMenu)}
        />
      {/snippet}
    </StickyHeader>

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
    open={Boolean(connection?.connectionIssue)}
    titleId="connection-issue-confirmation-title"
    title="Connection issue"
    description={connection?.connectionIssue?.message ?? 'An unknown connection issue occurred.'}
    confirmLabel="Retry"
    appearance="warning"
    onconfirm={async () => { await backendState.connectDevice(connection.deviceId); }}
    oncancel={() => {}}
  />

  <ConfirmDialog
    open={Boolean(connection.firmware && !connection.firmware.supported)}
    titleId="firmware-unsupported-confirmation-title"
    title="Firmware unsupported"
    description="Firmware {connection.firmware?.vescVersion} / {connection.firmware?.packageVersion} is not supported. Live values may be incomplete."
    confirmLabel="Continue"
    appearance="warning"
    onconfirm={() => {}}
    oncancel={disconnect}
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
