<script lang="ts">
  import { backendState } from '$lib/backend/state.svelte';
  import { formatDistanceM } from '$lib/features/measurements/format';

  const units = $derived(backendState.settings?.unitPreference ?? 'metric');
  const totals = $derived.by(() => {
    let sessions = 0;
    let elapsedMs = 0;
    let distanceM: number | null = backendState.savedDevices.length ? null : 0;
    for (const { summary } of backendState.savedDevices) {
      sessions += summary.sessionCount;
      elapsedMs += summary.totalRidingElapsedMs;
      if (summary.totalDistanceM !== null) distanceM = (distanceM ?? 0) + summary.totalDistanceM;
    }
    return { sessions, hours: elapsedMs / 3_600_000, distanceM };
  });
</script>

<dl aria-label="Session totals" class="grid grid-cols-3 divide-x divide-outline py-2">
  <div class="min-w-0 px-3">
    <dt class="text-xs text-muted">Total distance</dt>
    <dd class="mt-1 text-lg font-semibold tabular-nums">
      {backendState.snapshot ? formatDistanceM(totals.distanceM, units) : '—'}
    </dd>
  </div>
  <div class="min-w-0 px-3">
    <dt class="text-xs text-muted">Total hours</dt>
    <dd class="mt-1 text-lg font-semibold tabular-nums">
      {backendState.snapshot ? `${totals.hours.toFixed(1)} h` : '—'}
    </dd>
  </div>
  <div class="min-w-0 px-3">
    <dt class="text-xs text-muted">Total sessions</dt>
    <dd class="mt-1 text-lg font-semibold tabular-nums">
      {backendState.snapshot ? totals.sessions.toLocaleString() : '—'}
    </dd>
  </div>
</dl>
