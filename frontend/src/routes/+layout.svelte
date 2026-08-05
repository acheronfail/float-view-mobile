<script lang="ts">
  import 'uno.css';
  import '../app.css';
  import AppShell from '$lib/app/AppShell.svelte';
  let { children } = $props();
</script>

<AppShell>
  {@render children?.()}
</AppShell>
