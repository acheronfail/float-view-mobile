<script lang="ts">
  let { title, detail }: { title: string; detail?: string } = $props();
</script>

<h3
  class="sticky z-10 border-b border-b-accent-soft bg-canvas py-2 font-mono text-sm tracking-[0.10em] text-accent tabular-nums"
  style="top: var(--session-chart-header-top, 0px)"
>
  <div class="flex items-center justify-between">
    <span class="font-semibold">{title}</span>
    {#if detail}
      <span class="text-muted">{detail}</span>
    {/if}
  </div>
</h3>
