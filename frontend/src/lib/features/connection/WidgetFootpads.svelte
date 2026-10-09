<script lang="ts">
  import { footpadPresentation } from './footpads';
  import type { LiveSample } from '../../backend/contract';

  let {
    sample,
  }: {
    sample: LiveSample | null | undefined;
  } = $props();

  let fp = $derived(sample?.footpadState ?? 'unknown');
  const left = $derived(footpadPresentation(sample, 'left'));
  const right = $derived(footpadPresentation(sample, 'right'));
</script>

<section class="flex h-full min-h-0 flex-col overflow-hidden" aria-label={`Footpads: ${fp}`}>
  <h3 class="text-center text-xs font-bold tracking-wider text-muted uppercase">Footpads</h3>
  <div class="flex min-h-0 w-full grow flex-row items-center justify-center">
    <div class={`h-full w-1/2 rounded-l-full border border-outline ${left.background}`}></div>
    <div class={`h-full w-1/2 rounded-r-full border border-outline ${right.background}`}></div>
  </div>
</section>
