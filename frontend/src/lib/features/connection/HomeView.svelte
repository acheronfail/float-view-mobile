<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import AppHeaderSubtitle from '$lib/app/AppHeaderSubtitle.svelte';
  import { backendState } from '$lib/backend/state.svelte';
  import ListItem from '$lib/ui/ListItem.svelte';
  import SectionTitle from '../../ui/SectionTitle.svelte';
  import Spinner from '../../ui/Spinner.svelte';

  const SCAN_INTERVAL_MS = 10_000;
  const RETRY_INTERVAL_MS = 1_000;

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

  async function connect(deviceId: string) {
    if (await backendState.connectDevice(deviceId)) await goto('#/live');
  }

  function formatDeviceState(state: string) {
    return state.replaceAll('_', ' ').replace(/\b\w/g, (character) => character.toUpperCase());
  }

  // TODO: show connection strength with progress bar and dB
  // TODO: show uuid rather than just `discovered` or state or whatever that is
  // TODO: show overall stats below header but above scan results (total odometer sum, total ride time)
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
              <ListItem title={device.name} subtitle={formatDeviceState(device.state)}>
                {#snippet trailing()}
                  <button
                    class="rounded-md bg-accent px-3 py-1 text-on-accent disabled:opacity-50"
                    disabled={backendState.busy}
                    onclick={() => connect(device.deviceId)}>Connect</button
                  >
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
    <button class="rounded-lg bg-elevated px-4 py-2" onclick={() => backendState.connect()}>Retry</button>
  {:else}
    <p class="text-muted">Connecting to backend…</p>
  {/if}

  {#if backendState.error && backendState.status !== 'failed'}
    <p role="alert" class="text-warning">{backendState.error}</p>
  {/if}
</section>
