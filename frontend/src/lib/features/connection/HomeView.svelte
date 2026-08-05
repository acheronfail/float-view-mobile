<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import AppHeaderSubtitle from '$lib/app/AppHeaderSubtitle.svelte';
  import { backendState } from '$lib/backend/state.svelte';
  import type { DiscoveredDevice } from '$lib/backend/contract';
  import Icon from '$lib/ui/Icon.svelte';
  import type { IconName } from '$lib/ui/icon';
  import ListItem from '$lib/ui/ListItem.svelte';
  import NewBoardDialog from '$lib/ui/NewBoardDialog.svelte';
  import SectionTitle from '../../ui/SectionTitle.svelte';
  import Spinner from '../../ui/Spinner.svelte';

  const SCAN_INTERVAL_MS = 10_000;
  const RETRY_INTERVAL_MS = 1_000;
  let pendingDevice = $state<DiscoveredDevice | null>(null);

  onMount(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout> | undefined;

    const wait = (delayMs: number) =>
      new Promise<void>((resolve) => {
        timer = setTimeout(resolve, delayMs);
      });

    async function scanWhileActive() {
      while (active) {
        if (backendState.status === 'connected' && !backendState.scanning && !backendState.busy) {
          await backendState.scan();
          if (!active) return;
          await wait(SCAN_INTERVAL_MS);
        } else {
          await wait(RETRY_INTERVAL_MS);
        }
      }
    }

    void scanWhileActive();
    return () => {
      active = false;
      if (timer) clearTimeout(timer);
    };
  });

  async function chooseDevice(device: DiscoveredDevice) {
    if (backendState.busy) return;
    if (!device.savedDeviceId) {
      pendingDevice = device;
      return;
    }
    if (await backendState.connectDevice(device.deviceId, 'saved')) await goto('#/live');
  }

  function cancelNewBoard() {
    pendingDevice = null;
  }

  async function connectNewBoard(trackingMode: 'saved' | 'guest') {
    const device = pendingDevice;
    if (!device) return;
    pendingDevice = null;
    if (await backendState.connectDevice(device.deviceId, trackingMode)) {
      await goto('#/live');
    } else {
      pendingDevice = device;
    }
  }

  function signalBarCount(rssi: number | null | undefined) {
    if (rssi === null || rssi === undefined || !Number.isFinite(rssi)) return 0;
    if (rssi >= -60) return 4;
    if (rssi >= -70) return 3;
    if (rssi >= -80) return 2;
    return 1;
  }

  function signalIconName(rssi: number | null | undefined): IconName {
    switch (signalBarCount(rssi)) {
      case 1:
        return 'signal-low';
      case 2:
        return 'signal-medium';
      case 3:
      case 4:
        return 'signal-high';
      default:
        return 'signal';
    }
  }

  function formatSignalStrength(rssi: number | null | undefined) {
    return rssi === null || rssi === undefined || !Number.isFinite(rssi) ? 'Unavailable' : `${rssi} dBm`;
  }
</script>

<AppHeaderSubtitle owner="/">
  <span aria-live="polite" class="inline-flex items-center gap-2">
    <span class="flex flex-row items-center gap-2">
      <Spinner />
      Scanning…
    </span>
  </span>
</AppHeaderSubtitle>

<section aria-label="VESC devices" class="flex flex-col gap-4 py-2">
  {#if backendState.status === 'connected'}
    {#if backendState.discoveredDevices.length}
      <div class="flex flex-col gap-2">
        <SectionTitle title="Discovered boards" />
        <ul class="flex flex-col gap-2">
          {#each backendState.discoveredDevices as device (device.deviceId)}
            <li>
              <ListItem
                title={device.name || device.deviceId}
                subtitle={device.deviceId}
                onactivate={() => chooseDevice(device)}
                disabled={backendState.busy}
              >
                {#snippet trailing()}
                  <span class="flex items-center gap-2">
                    <span
                      aria-label={`Signal strength: ${formatSignalStrength(device.rssi)}`}
                      class="inline-flex shrink-0 items-center gap-1 text-accent"
                      role="img"
                    >
                      <Icon name={signalIconName(device.rssi)} class="text-3xl" />
                      <span class="text-xs whitespace-nowrap text-muted tabular-nums">
                        {formatSignalStrength(device.rssi)}
                      </span>
                    </span>
                  </span>
                {/snippet}
              </ListItem>
            </li>
          {/each}
        </ul>
      </div>
    {:else}
      <p class="text-center text-muted">Looking for VESC devices…</p>
    {/if}
  {:else if backendState.status === 'failed'}
    <p role="alert" class="text-warning">{backendState.error}</p>
    <button type="button" class="rounded-lg bg-elevated px-4 py-2" onclick={() => backendState.connect()}>Retry</button>
  {:else}
    <p class="text-muted">Connecting to backend…</p>
  {/if}

  {#if backendState.error && backendState.status !== 'failed'}
    <p role="alert" class="text-warning">{backendState.error}</p>
  {/if}
</section>

<NewBoardDialog
  open={pendingDevice !== null}
  boardName={pendingDevice?.name || 'This board'}
  busy={backendState.busy}
  oncancel={cancelNewBoard}
  onguest={() => connectNewBoard('guest')}
  onsave={() => connectNewBoard('saved')}
/>
