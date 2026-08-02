<script lang="ts">
  let { title }: { title: string } = $props();
</script>

<h3
  class="sticky top-0 z-10 border-b border-b-accent-soft bg-canvas py-2 font-mono text-sm font-semibold tracking-[0.10em] text-accent tabular-nums"
>
  {title}
</h3>
