<script lang="ts">
  import { page } from '$app/state';
  import SessionView from '$lib/features/sessions/SessionView.svelte';

  const sessionId = $derived(page.params.sessionId ?? '');
</script>

{#if sessionId.length > 0 && sessionId.length <= 512}
  <SessionView {sessionId} />
{:else}
  <section class="rounded-2xl border border-outline bg-surface p-5">
    <p role="alert" class="mt-3 text-warning">This session identifier is invalid.</p>
  </section>
{/if}
