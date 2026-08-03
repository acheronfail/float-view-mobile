<script lang="ts">
  import { goto } from '$app/navigation';
  import PageHeader from '$lib/ui/PageHeader.svelte';
  import { bottomTabForRoute } from '$lib/app/navigation';
  import MenuItem from '$lib/ui/MenuItem.svelte';
  import { backendState } from '$lib/backend/state.svelte';
  import { reportFrontendPerformance } from '$lib/backend/performance-log';
  import SessionList from '$lib/features/sessions/SessionList.svelte';

  const pageHeader = bottomTabForRoute('/sessions')!;
  let menuButton = $state<HTMLButtonElement>();
  let importInput = $state<HTMLInputElement>();
  let sessionsHeaderHeight = $state(0);

  function chooseImport(closeMenu: () => void) {
    closeMenu();
    importInput?.click();
  }

  async function importSession(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    reportFrontendPerformance('session-import-file', {
      hasFile: Boolean(file),
      bytes: file?.size ?? 0,
      type: file?.type || 'unknown',
    });
    input.value = '';
    if (!file) return;
    const sessionId = await backendState.importSession(file);
    if (sessionId) await goto(`#/sessions/${encodeURIComponent(sessionId)}`);
  }
</script>

<section
  aria-labelledby="sessions-heading"
  class="space-y-4"
  style={`--section-header-top: calc(max(1.5rem, env(safe-area-inset-top, 0px)) + ${sessionsHeaderHeight}px)`}
>
  <PageHeader
    bind:button={menuButton}
    bind:headerHeight={sessionsHeaderHeight}
    headingId="sessions-heading"
    title={pageHeader.title}
    subtitle={pageHeader.subtitle}
    sticky
    menuLabel="Session actions"
  >
    {#snippet menu(closeMenu)}
      <MenuItem
        label="Import session"
        disabled={backendState.busy || backendState.sessionImporting}
        onactivate={() => chooseImport(closeMenu)}
      />
    {/snippet}
  </PageHeader>

  <input bind:this={importInput} class="hidden" type="file" accept="*/*" onchange={importSession} />
  {#if backendState.sessionImporting}
    <p class="text-muted" role="status">Importing session…</p>
  {/if}
  {#if backendState.error}
    <p class="text-warning" role="alert">{backendState.error}</p>
  {/if}
  <SessionList />
</section>
