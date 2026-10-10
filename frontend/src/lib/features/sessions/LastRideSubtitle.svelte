<script lang="ts">
  import { onMount } from 'svelte';
  import { backendState } from '$lib/backend/state.svelte';
  import { lastRideLabel, latestRideAt } from './last-ride';

  let now = $state(Date.now());
  const latest = $derived(latestRideAt(backendState.savedDevices));

  onMount(() => {
    const refresh = () => (now = Date.now());
    refresh();
    const timer = setInterval(refresh, 30_000);
    window.addEventListener('focus', refresh);
    document.addEventListener('visibilitychange', refresh);
    return () => {
      clearInterval(timer);
      window.removeEventListener('focus', refresh);
      document.removeEventListener('visibilitychange', refresh);
    };
  });
</script>

{#if latest !== null}
  <time datetime={new Date(latest).toISOString()} title={new Date(latest).toLocaleString()}>
    {lastRideLabel(latest, now)}
  </time>
{:else if backendState.snapshot}
  {lastRideLabel(null, now)}
{:else}
  Loading rides…
{/if}
