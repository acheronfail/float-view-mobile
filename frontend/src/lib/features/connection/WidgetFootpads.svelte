<script lang="ts">
  import type { LiveSample } from '../../backend/contract';

  let {
    sample,
  }: {
    sample: LiveSample | null | undefined;
  } = $props();

  let fp = $derived(sample?.footpadState ?? 'unknown');
  let adc1 = $derived(fp === 'both' || fp === 'left');
  let adc2 = $derived(fp === 'both' || fp === 'right');

  let shouldAlert = $derived((sample?.speedKph ?? 0) > 2);

  let adc1Bg = $derived(adc1 ? 'bg-success' : shouldAlert ? 'bg-danger' : 'bg-surface');
  let adc2Bg = $derived(adc2 ? 'bg-success' : shouldAlert ? 'bg-danger' : 'bg-surface');
</script>

<section class="flex h-full min-h-0 flex-col overflow-hidden" aria-labelledby="temperatures-title">
  <h3 class="text-center text-xs font-bold tracking-wider text-muted uppercase">Footpads</h3>
  <div class="flex grow flex-row items-center justify-center">
    <div class={`h-full w-1/2 rounded-l-full border border-outline ${adc1Bg}`}></div>
    <div class={`h-full w-1/2 rounded-r-full border border-outline ${adc2Bg}`}></div>
  </div>
</section>
