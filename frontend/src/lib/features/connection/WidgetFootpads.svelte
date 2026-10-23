<script lang="ts">
  import { footpadPresentation } from './footpads';
  import type { LiveSample } from '../../backend/contract';

  let {
    sample,
    compact = false,
    expanded = false,
  }: {
    compact?: boolean;
    expanded?: boolean;
    sample: LiveSample | null | undefined;
  } = $props();

  let fp = $derived(sample?.footpadState ?? 'unknown');
  const left = $derived(footpadPresentation(sample, 'left'));
  const right = $derived(footpadPresentation(sample, 'right'));
</script>

<section
  class:expanded
  class="flex h-full min-h-0 flex-col justify-center overflow-hidden"
  aria-label={`Footpads: ${fp}`}
>
  <h3 class={compact ? 'live-widget-title' : 'text-center text-xs font-bold text-muted uppercase'}>Footpads</h3>
  <div
    class={`footpad-bar flex min-h-0 w-full flex-row items-center justify-center ${compact ? 'h-5 shrink-0' : 'grow'}`}
  >
    <div class={`h-full w-1/2 rounded-l-full border border-outline ${left.background}`}></div>
    <div class={`h-full w-1/2 rounded-r-full border border-outline ${right.background}`}></div>
  </div>
</section>

<style>
  @container custom-widget (min-width: 120px) and (min-height: 65px) {
    .expanded .footpad-bar {
      height: 28px;
      margin-top: 4px;
    }
  }
</style>
