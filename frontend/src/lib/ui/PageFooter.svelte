<script lang="ts">
  import type { Snippet } from 'svelte';

  let { label, children }: { label: string; children: Snippet } = $props();
</script>

<footer aria-label={label} class="border-t border-outline pt-4 pb-2 text-center text-xs text-muted">
  {@render children()}
</footer>
