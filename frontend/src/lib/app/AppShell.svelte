<script lang="ts">
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { backendState } from '$lib/backend/state.svelte';
  import BottomTabBar from './BottomTabBar.svelte';
  import { bottomTabForRoute } from './navigation';
  import PageHeader from '$lib/ui/PageHeader.svelte';
  import { browserThemeEnvironment, ThemeController } from './theme';

  let { children }: { children: Snippet } = $props();
  let themeController: ThemeController | undefined;
  const activeBottomTab = $derived(bottomTabForRoute(page.route.id));
  const liveRoute = $derived(page.route.id === '/live');
  const sessionsRoute = $derived(page.route.id === '/sessions');
  const pageHeader = $derived(activeBottomTab);

  onMount(() => {
    themeController = new ThemeController(browserThemeEnvironment());
    const stopTheme = themeController.start();
    const handleNativeImportStarted = () => {
      backendState.sessionImporting = true;
      backendState.error = null;
      void goto('#/sessions');
    };
    const handleNativeImport = (event: Event) => {
      backendState.sessionImporting = false;
      const sessionId = (event as CustomEvent<{ sessionId?: unknown }>).detail?.sessionId;
      if (typeof sessionId === 'string' && sessionId) {
        void goto(`#/sessions/${encodeURIComponent(sessionId)}`);
      }
    };
    const handleNativeImportFailure = (event: Event) => {
      backendState.sessionImporting = false;
      const message = (event as CustomEvent<{ message?: unknown }>).detail?.message;
      backendState.error = typeof message === 'string' && message ? message : 'Session import failed';
    };
    window.addEventListener('floatview:session-import-started', handleNativeImportStarted);
    window.addEventListener('floatview:session-imported', handleNativeImport);
    window.addEventListener('floatview:session-import-failed', handleNativeImportFailure);
    const preference = backendState.settings?.themePreference;
    if (preference) themeController.reconcile(preference);
    void backendState.connect();
    return () => {
      window.removeEventListener('floatview:session-import-started', handleNativeImportStarted);
      window.removeEventListener('floatview:session-imported', handleNativeImport);
      window.removeEventListener('floatview:session-import-failed', handleNativeImportFailure);
      stopTheme();
      themeController = undefined;
      backendState.close();
    };
  });

  $effect(() => {
    const preference = backendState.settings?.themePreference;
    if (!preference) return;
    themeController?.reconcile(preference);
  });

  $effect(() => {
    if (backendState.status === 'connected' && backendState.snapshot?.activeConnection && page.route.id === '/') {
      void goto('#/live', { replaceState: true });
    }
  });
</script>

<main class="min-h-screen bg-canvas text-foreground">
  <div
    class="app-safe-area mx-auto flex flex-col"
    class:min-h-screen={!liveRoute}
    class:h-screen={liveRoute}
    class:min-h-0={liveRoute}
    class:max-w-3xl={!liveRoute}
    class:max-w-5xl={liveRoute}
    class:app-safe-area-with-tabs={activeBottomTab}
    class:app-safe-area-without-tabs={!activeBottomTab}
  >
    {#if !liveRoute && !sessionsRoute && pageHeader}
      <PageHeader headingId="app-title" title={pageHeader.title} subtitle={pageHeader.subtitle} />
    {/if}

    <div class="flex flex-1 flex-col" class:min-h-0={liveRoute}>
      {@render children()}
    </div>
  </div>
  <BottomTabBar routeId={page.route.id} />
</main>
