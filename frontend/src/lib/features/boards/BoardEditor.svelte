<script lang="ts">
  import { getContext, onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { APP_HEADER_CONTEXT, type AppHeaderContext } from '$lib/app/header';
  import { backendState } from '$lib/backend/state.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import {
    normalizedCustomName,
    SerialSaveQueue,
    validParallelCount,
    validPreferredCanId,
    validSeriesCount,
  } from './autosave';

  let {
    savedDeviceId,
    embedded = false,
    onclose,
  }: { savedDeviceId: string; embedded?: boolean; onclose?: () => void } = $props();
  let customDisplayName = $state('');
  let seriesCount = $state<number | undefined>();
  let parallelCount = $state<number | undefined>();
  let motor = $state('');
  let preferredCanId = $state<number | undefined>();
  let preferredBmsCanId = $state<number | undefined>();
  const appHeader = getContext<AppHeaderContext>(APP_HEADER_CONTEXT);
  const headerOwner = '/sessions/boards/[savedDeviceId]';
  const saveQueue = new SerialSaveQueue();

  const device = $derived(backendState.savedDevices.find((value) => value.savedDeviceId === savedDeviceId));
  let name = $derived(
    (customDisplayName.length ? customDisplayName : (device?.advertisedName ?? device?.bluetoothIdentifier)) ?? 'Board'
  );

  $effect(() => {
    if (embedded) return;
    appHeader.set(headerOwner, {
      title: name,
      backHref: '#/sessions',
      backLabel: 'Sessions',
    });
  });
  onDestroy(() => {
    if (!embedded) appHeader.clear(headerOwner);
  });

  onMount(() => {
    let active = true;
    void backendState.openBoardProfile(savedDeviceId).then((opened) => {
      if (!active) return;
      if (!opened) {
        if (embedded) onclose?.();
        else void goto('#/sessions', { replaceState: true });
        return;
      }
      customDisplayName = device?.customDisplayName ?? '';
      const values = backendState.selectedBoardProfile?.values;
      seriesCount = typeof values?.seriesCount === 'number' ? values.seriesCount : undefined;
      parallelCount = typeof values?.parallelCount === 'number' ? values.parallelCount : undefined;
      motor = typeof values?.motor === 'string' ? values.motor : '';
      preferredCanId = typeof values?.preferredCanId === 'number' ? values.preferredCanId : undefined;
      preferredBmsCanId = typeof values?.preferredBmsCanId === 'number' ? values.preferredBmsCanId : undefined;
    });
    return () => {
      active = false;
      backendState.closeBoardProfile();
    };
  });

  function enqueue(action: () => Promise<void>) {
    saveQueue.enqueue(action);
  }

  function saveName() {
    const next = normalizedCustomName(customDisplayName);
    if (next === (device?.customDisplayName ?? null)) return;
    enqueue(() => backendState.updateSavedDeviceName(savedDeviceId, next));
  }

  function saveProfile(patch: Record<string, unknown>) {
    enqueue(() => backendState.updateBoardProfile(patch));
  }

  export async function flush() {
    await saveQueue.flush();
  }
</script>

<section aria-labelledby="app-title">
  {#if backendState.selectedBoardProfile && device}
    {#if backendState.selectedBoardProfile.missingFields.length}
      <p class="mt-3 text-sm text-warning">
        <Icon name="triangle-alert" class="mr-1 text-xl" />
        Missing: {backendState.selectedBoardProfile.missingFields.join(', ')}
      </p>
    {/if}
    <div class="mt-4 grid gap-3 sm:grid-cols-2">
      <label class="text-sm sm:col-span-2">
        <span class="mb-1 block text-muted">Display name</span>
        <input
          class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
          type="text"
          maxlength="128"
          bind:value={customDisplayName}
          placeholder={device.advertisedName ?? device.bluetoothIdentifier ?? 'Board'}
          onblur={saveName}
        />
      </label>
      <label class="text-sm sm:col-span-2">
        <span class="mb-1 block text-muted">Preferred BMS CAN ID (optional)</span>
        <input
          class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
          type="number"
          min="0"
          max="253"
          inputmode="numeric"
          bind:value={preferredBmsCanId}
          aria-describedby="preferred-bms-can-help"
          onblur={() => {
            if (preferredBmsCanId === undefined) saveProfile({ preferredBmsCanId: null });
            else if (validPreferredCanId(preferredBmsCanId)) saveProfile({ preferredBmsCanId });
          }}
        />
        <span id="preferred-bms-can-help" class="mt-1 block text-xs text-muted">
          Applies immediately to BMS discovery only. Leave blank to resume automatic BMS selection.
        </span>
        {#if preferredBmsCanId !== undefined && !validPreferredCanId(preferredBmsCanId)}
          <span role="alert" class="mt-1 block text-xs text-warning"> Enter a whole CAN ID from 0 to 253. </span>
        {/if}
      </label>
      <label class="text-sm sm:col-span-2">
        <span class="mb-1 block text-muted">Preferred CAN ID (optional)</span>
        <input
          class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
          type="number"
          min="0"
          max="253"
          inputmode="numeric"
          bind:value={preferredCanId}
          aria-describedby="preferred-can-help"
          onblur={() => {
            if (preferredCanId === undefined) saveProfile({ preferredCanId: null });
            else if (validPreferredCanId(preferredCanId)) saveProfile({ preferredCanId });
          }}
        />
        <span id="preferred-can-help" class="mt-1 block text-xs text-muted">
          Used on the next VESC Express connection. Leave blank for automatic discovery.
        </span>
        {#if preferredCanId !== undefined && !validPreferredCanId(preferredCanId)}
          <span role="alert" class="mt-1 block text-xs text-warning"> Enter a whole CAN ID from 0 to 253. </span>
        {/if}
      </label>
      <div class="rounded-md border border-outline bg-elevated px-3 py-2 text-sm sm:col-span-2">
        <span class="block text-muted">Bluetooth identifier</span>
        <code class="mt-1 block break-all text-foreground"
          >{device.bluetoothIdentifier ?? 'Not locally associated'}</code
        >
      </div>
      <label class="text-sm">
        <span class="mb-1 block text-muted">Battery series cells (S)</span>
        <input
          class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
          type="number"
          min="1"
          max="36"
          bind:value={seriesCount}
          onblur={() => {
            if (validSeriesCount(seriesCount)) saveProfile({ seriesCount });
          }}
        />
      </label>
      <label class="text-sm">
        <span class="mb-1 block text-muted">Battery parallel cells (P)</span>
        <input
          class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
          type="number"
          min="1"
          max="20"
          bind:value={parallelCount}
          onblur={() => {
            if (validParallelCount(parallelCount)) saveProfile({ parallelCount });
          }}
        />
      </label>
      <label class="text-sm sm:col-span-2">
        <span class="mb-1 block text-muted">Motor</span>
        <select
          class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
          bind:value={motor}
          onchange={() => {
            if (motor) saveProfile({ motor });
          }}
        >
          <option value="">Choose a motor</option>
          <option value="hypercore_xr">Hypercore (XR)</option>
          <option value="superflux_hs">Superflux HS</option>
          <option value="custom">Custom / other</option>
        </select>
      </label>
    </div>
  {:else}
    <p class="text-muted">Loading board configuration…</p>
  {/if}
  {#if backendState.error}<p role="alert" class="mt-3 text-warning">{backendState.error}</p>{/if}
</section>
