<script lang="ts">
  import SectionTitle from '../../ui/SectionTitle.svelte';

  let { title, items }: { title: string; items: [string, string][] } = $props();
</script>

<section aria-label={title} class="flex flex-col gap-2">
  <SectionTitle {title} />
  <dl class="divide-y divide-outline overflow-hidden rounded border border-outline font-mono">
    {#each items as item (item[0])}
      <div class="grid grid-cols-[max-content_minmax(0,1fr)] items-center px-2 py-1 odd:bg-surface">
        <dt class="text-xs text-muted uppercase">{item[0]}:</dt>
        <dd class="min-w-0 text-right text-xs font-medium break-words">{item[1]}</dd>
      </div>
    {/each}
  </dl>
</section>
