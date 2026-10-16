<script lang="ts">
  import { routeSearchParams } from '$lib/app/navigation';
  import { page } from '$app/state';
  import SessionView from '$lib/features/sessions/SessionView.svelte';

  const sessionId = $derived(page.params.sessionId ?? '');
  const query = $derived(routeSearchParams(page.url));
  const fromLive = $derived(query.get('from') === 'live');
  const boardSavedDeviceId = $derived(query.get('board'));
</script>

{#if sessionId.length > 0 && sessionId.length <= 512}
  {#key sessionId}
    <SessionView {sessionId} {boardSavedDeviceId} {fromLive} />
  {/key}
{:else}
  <section class="rounded-2xl border border-outline bg-surface p-5">
    <p role="alert" class="mt-3 text-warning">This session identifier is invalid.</p>
  </section>
{/if}
