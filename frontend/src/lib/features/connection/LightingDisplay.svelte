<script lang="ts">
  import './widget-title.css';
  import ToggleIndicator from '$lib/ui/ToggleIndicator.svelte';
  import { liveValueCatalog } from './live-values';
  import type { LightingControl, lightingBox } from './lighting';
  let {
    field,
    lighting,
    toggling = false,
    expanded = false,
  }: {
    field: LightingControl;
    lighting: ReturnType<typeof lightingBox>;
    toggling?: boolean;
    expanded?: boolean;
  } = $props();
</script>

<div
  class="flex flex-col items-center justify-center gap-1"
  class:expanded
  class:opacity-50={lighting.disabled && !lighting.pending}
>
  <span class="live-widget-title max-w-full"
    >{lighting.pending || toggling ? 'Pending…' : liveValueCatalog[field].label}</span
  >
  {#if lighting.checked != null}<span class="lighting-switch inline-flex"
      ><ToggleIndicator checked={lighting.checked} /></span
    >{:else}<span class="text-xs text-muted">{lighting.status}</span>{/if}
</div>

<style>
  @container custom-widget (min-width: 90px) and (min-height: 65px) {
    .expanded {
      gap: 8px;
    }
    .expanded .lighting-switch {
      transform: scale(1.35);
      margin: 4px;
    }
  }
</style>
