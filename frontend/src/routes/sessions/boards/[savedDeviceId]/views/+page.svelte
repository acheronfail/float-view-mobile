<script lang="ts">
  import { page } from '$app/state';
  import BoardViewsEditor from '$lib/features/connection/BoardViewsEditor.svelte';

  const savedDeviceId = $derived(page.params.savedDeviceId ?? '');
</script>

{#if savedDeviceId.length > 0 && savedDeviceId.length <= 512}
  {#key savedDeviceId}
    <BoardViewsEditor {savedDeviceId} />
  {/key}
{:else}
  <p role="alert">This board identifier is invalid.</p>
{/if}
