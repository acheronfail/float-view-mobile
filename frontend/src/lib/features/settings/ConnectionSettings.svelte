<script lang="ts">
  import { backendState } from '$lib/backend/state.svelte';
  import SectionTitle from '$lib/ui/SectionTitle.svelte';
  import Permissions from './Permissions.svelte';
  const inputClass = 'mt-1 block w-full rounded-lg border border-outline bg-elevated px-3 py-2 text-foreground';
</script>

<Permissions />

{#if backendState.settings}
  <section>
    <SectionTitle title="Reconnecting" />
    <label class="text-sm text-muted"
      >Reconnect window (minutes)
      <input
        class={inputClass}
        type="number"
        inputmode="numeric"
        min="0"
        max="60"
        step="1"
        value={backendState.settings.reconnectDurationMs / 60_000}
        onchange={(event) =>
          backendState.updateSessionSettings({
            reconnectDurationMs: Math.round(Number(event.currentTarget.value) * 60_000),
          })}
      />
    </label>
  </section>
{/if}
