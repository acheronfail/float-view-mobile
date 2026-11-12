<script lang="ts">
  import { page } from '$app/state';
  import { routeSearchParams } from '$lib/app/navigation';
  import BoardViewsEditor from '$lib/features/connection/BoardViewsEditor.svelte';

  const savedDeviceId = $derived(page.params.savedDeviceId ?? '');
  const query = $derived(routeSearchParams(page.url));
</script>

{#if savedDeviceId.length > 0 && savedDeviceId.length <= 512}
  {#key savedDeviceId}
    <BoardViewsEditor {savedDeviceId} fromLive={query.get('from') === 'live'} />
  {/key}
{:else}
  <p role="alert">This board identifier is invalid.</p>
{/if}
