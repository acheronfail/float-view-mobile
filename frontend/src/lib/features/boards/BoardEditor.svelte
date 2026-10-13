<script lang="ts">
  import SegmentedControl from '$lib/ui/SegmentedControl.svelte';
  import { getContext, onDestroy, onMount, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { APP_HEADER_CONTEXT, type AppHeaderContext } from '$lib/app/header';
  import { backendState } from '$lib/backend/state.svelte';
  import BatteryFields from './BatteryFields.svelte';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  import Icon from '$lib/ui/Icon.svelte';
  import { normalizedCustomName, SerialSaveQueue, validPreferredCanId } from './autosave';

  let {
    savedDeviceId,
    embedded = false,
    batteryOnly = false,
    onclose,
  }: { savedDeviceId: string; embedded?: boolean; batteryOnly?: boolean; onclose?: () => void } = $props();
  let customDisplayName = $state('');
  let seriesCount = $state<number | undefined>();
  let parallelCount = $state<number | undefined>();
  let batteryCellProfileId = $state('');
  let batteryPresetId = $state('');
  let batteryCapacityAh = $state<number | undefined>();
  let pendingSaves = $state(0);
  let batteryFields = $state<BatteryFields | undefined>();
  let batteryMinimumCellVoltageV = $state<number | undefined>();
  let motor = $state('');
  let preferredCanId = $state<number | undefined>();
  let preferredBmsCanId = $state<number | undefined>();
  const appHeader = getContext<AppHeaderContext>(APP_HEADER_CONTEXT);
  const headerOwner = '/sessions/boards/[savedDeviceId]';
  const saveQueue = new SerialSaveQueue();
  let active = false;
  let loaded = $state(false);
  let saveError = $state<string | null>(null);
  let failedPatch: Record<string, unknown> = {};
  let discardDrafts = false;

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
    active = true;
    void loadProfile();
    return () => {
      active = false;
      backendState.closeBoardProfile();
    };
  });

  function loadProfile() {
    return backendState.openBoardProfile(savedDeviceId).then((opened) => {
      if (!active) return;
      if (!opened) {
        if (!backendState.boardProfileNotFound) return;
        if (embedded) onclose?.();
        else void goto('#/sessions', { replaceState: true });
        return;
      }
      loaded = true;
      customDisplayName = device?.customDisplayName ?? '';
      const values = backendState.selectedBoardProfile?.values;
      seriesCount = typeof values?.seriesCount === 'number' ? values.seriesCount : undefined;
      parallelCount = typeof values?.parallelCount === 'number' ? values.parallelCount : undefined;
      batteryCellProfileId = values?.batteryCellProfileId ?? '';
      batteryPresetId = values?.batteryPresetId ?? '';
      batteryCapacityAh = values?.batteryCapacityAh ?? undefined;
      batteryMinimumCellVoltageV = values?.batteryMinimumCellVoltageV ?? undefined;
      motor = typeof values?.motor === 'string' ? values.motor : '';
      preferredCanId = typeof values?.preferredCanId === 'number' ? values.preferredCanId : undefined;
      preferredBmsCanId = typeof values?.preferredBmsCanId === 'number' ? values.preferredBmsCanId : undefined;
    });
  }

  function enqueue(action: () => Promise<void>) {
    const ownerId = savedDeviceId;
    saveQueue.enqueue(async () => {
      if (
        !active ||
        !loaded ||
        savedDeviceId !== ownerId ||
        !device ||
        backendState.selectedBoardProfile?.savedDeviceId !== savedDeviceId
      )
        return;
      await action();
    });
  }

  function saveName() {
    const next = normalizedCustomName(customDisplayName);
    if (next === (device?.customDisplayName ?? null)) return;
    enqueue(() => backendState.updateSavedDeviceName(savedDeviceId, next));
  }

  function saveProfile(patch: Record<string, unknown>) {
    enqueue(async () => {
      pendingSaves++;
      const saved = await backendState.updateBoardProfile(savedDeviceId, patch);
      pendingSaves--;
      if (!active) return;
      if (saved) {
        if (patch.batteryPresetId) {
          for (const key of [
            'batteryPresetId',
            'seriesCount',
            'parallelCount',
            'batteryCellProfileId',
            'batteryCapacityAh',
            'batteryMinimumCellVoltageV',
          ])
            delete failedPatch[key];
        }
        for (const key of Object.keys(patch)) delete failedPatch[key];
        if (!Object.keys(failedPatch).length) saveError = null;
      } else {
        Object.assign(failedPatch, patch);
        saveError = backendState.error ?? 'Settings could not be saved. Please try again.';
      }
    });
  }

  async function discardFailedChanges() {
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    await tick();
    await saveQueue.flush();
    failedPatch = {};
    saveError = null;
    discardDrafts = true;
    onclose?.();
  }

  export async function flush() {
    await saveQueue.flush();
    if (!saveError && !discardDrafts) batteryFields?.flushDrafts();
    await saveQueue.flush();
    return saveError === null;
  }
</script>

<section aria-label={batteryOnly ? 'Battery settings' : 'Board settings'}>
  {#if loaded && backendState.selectedBoardProfile?.savedDeviceId === savedDeviceId && device}
    {#if !batteryOnly && backendState.selectedBoardProfile.missingFields.length}
      <p class="mt-3 text-sm text-warning">
        <Icon name="triangle-alert" class="mr-1 text-xl" />
        Missing: {backendState.selectedBoardProfile.missingFields.join(', ')}
      </p>
    {/if}
    {#if batteryOnly}
      <p class="text-sm text-muted">Set up your battery for a more accurate charge estimate.</p>
    {/if}
    <div class="mt-4 grid gap-3 sm:grid-cols-2">
      {#if !batteryOnly}
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
      {/if}
      <BatteryFields
        bind:this={batteryFields}
        savedValues={backendState.selectedBoardProfile.values}
        bind:seriesCount
        bind:parallelCount
        bind:batteryPresetId
        bind:batteryCapacityAh
        presets={backendState.snapshot?.batteryPresets ?? []}
        bind:batteryCellProfileId
        bind:batteryMinimumCellVoltageV
        profiles={backendState.snapshot?.batteryCellProfiles ?? []}
        {saveProfile}
      />
      {#if !batteryOnly}
        <div class="text-sm sm:col-span-2">
          <span class="mb-1 block text-muted">Motor</span>
          <SegmentedControl
            label="Motor"
            options={[
              { value: 'hypercore_xr', label: 'Hypercore (XR)' },
              { value: 'superflux_hs', label: 'Superflux HS' },
              { value: 'custom', label: 'Custom / other' },
            ]}
            value={motor}
            onchange={(value) => {
              motor = value;
              if (motor) saveProfile({ motor });
            }}
          />
        </div>
      {/if}
    </div>
  {:else}
    <p class="text-muted">Loading board configuration…</p>
  {/if}
  {#if pendingSaves}
    <p role="status" class="mt-3 text-sm text-muted">Saving settings…</p>
  {/if}
  {#if saveError}
    <p role="alert" class="mt-3 text-warning">{saveError}</p>
    {#if embedded}
      <button
        type="button"
        class="mt-2 touch-manipulation rounded border border-outline px-3 py-2 text-sm"
        onpointerdown={(event) => activateFromPointer(event, discardFailedChanges)}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, discardFailedChanges)}
      >
        Close without saving failed changes
      </button>
    {/if}
  {:else if backendState.error}
    <p role="alert" class="mt-3 text-warning">{backendState.error}</p>
    <button
      type="button"
      class="mt-3 touch-manipulation rounded-lg bg-elevated px-4 py-2"
      onpointerdown={(event) => activateFromPointer(event, loadProfile)}
      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, loadProfile)}>Retry</button
    >
  {/if}
</section>
