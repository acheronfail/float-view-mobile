<script lang="ts">
  import type { LiveSample } from '$lib/backend/contract';
  import LiveValueBox from './LiveValueBox.svelte';
  let { sample }: { sample: LiveSample | null | undefined } = $props();
</script>

<div class="grid grid-cols-4 gap-1.5" aria-label="Live number widgets">
  {#each [0, 1, 2, 3] as slot (slot)}
    <LiveValueBox {sample} {slot} />
  {/each}
</div>
