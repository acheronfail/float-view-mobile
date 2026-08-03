<script lang="ts">
  import { goto } from '$app/navigation';
  import { tick } from 'svelte';
  import { backendState } from '$lib/backend/state.svelte';
  import BoardEditor from '$lib/features/boards/BoardEditor.svelte';
  import ControllerSelection from '$lib/features/connection/ControllerSelection.svelte';
  import DetailedView from '$lib/features/connection/DetailedView.svelte';
  import LiveView from '$lib/features/connection/LiveView.svelte';
  import { shouldConfirmDisconnect } from '$lib/features/connection/live';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import BottomSheet from '$lib/ui/BottomSheet.svelte';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import MenuItem from '$lib/ui/MenuItem.svelte';
  import StickyHeader from '$lib/ui/StickyHeader.svelte';

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

    <div class="shrink-0 space-y-2">
      {#if connection.firmware && !connection.firmware.supported}
        <div class="rounded-xl border border-warning/50 bg-warning/10 p-3 text-sm text-warning">
          Firmware {connection.firmware.vescVersion} / {connection.firmware.packageVersion} is not supported. Live values
          may be incomplete.
        </div>
      {/if}
      {#if connection.connectionIssue}
        <div class="rounded-xl border border-warning/50 bg-warning/10 p-3 text-sm text-warning">
          <p>{connection.connectionIssue.message}</p>
          {#if connection.state === 'failed'}
            <button
              type="button"
              class="mt-3 touch-manipulation rounded-md border border-warning/60 px-3 py-2 font-medium"
              disabled={backendState.busy}
              onpointerdown={(event) =>
                activateFromPointer(event, async () => {
                  await backendState.connectDevice(connection.deviceId);
                })}
              onclick={(event) =>
                activateFromKeyboardOrAssistiveClick(event, async () => {
                  await backendState.connectDevice(connection.deviceId);
                })}
            >
              {backendState.busy ? 'Retrying…' : 'Retry connection'}
            </button>
          {/if}
        </div>
      {/if}
    </div>

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

<ConfirmDialog
  open={disconnectConfirmationOpen}
  titleId="disconnect-confirmation-title"
  title="Disconnect board?"
  description="The board is not idle. This will stop the active session and disconnect the board."
  confirmLabel="Disconnect"
  destructive
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
