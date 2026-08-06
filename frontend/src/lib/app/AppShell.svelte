<script lang="ts">
  import type { Snippet } from 'svelte';
  import { onMount, setContext } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { backendState } from '$lib/backend/state.svelte';
  import BottomTabBar from './BottomTabBar.svelte';
  import { bottomTabForRoute } from './navigation';
  import { APP_HEADER_CONTEXT, type AppHeaderConfig, type AppHeaderContext, type AppHeaderPatch } from './header';
  import PageHeader from '$lib/ui/PageHeader.svelte';
  import { browserThemeEnvironment, ThemeController } from './theme';
  import { connectionRedirect } from './connection-navigation';

  let { children }: { children: Snippet } = $props();
  let themeController = $state<ThemeController | undefined>(undefined);
  let appHeaderHeight = $state(0);
  let appHeaderButton = $state<HTMLButtonElement>();
  let headerRegistration = $state<{ owner: string; patch: AppHeaderPatch } | undefined>();
  const activeBottomTab = $derived(bottomTabForRoute(page.route.id));
  const liveRoute = $derived(page.route.id === '/live');

  const headerContext: AppHeaderContext = {
    set(owner, patch) {
      headerRegistration = { owner, patch };
    },
    clear(owner) {
      if (headerRegistration?.owner === owner) headerRegistration = undefined;
    },
    focusMenu() {
      appHeaderButton?.focus();
    },
  };
  setContext(APP_HEADER_CONTEXT, headerContext);

  const routeHeader = $derived.by((): AppHeaderConfig => {
    if (activeBottomTab) return activeBottomTab;

    switch (page.route.id) {
      case '/live':
        return { title: 'Live board', subtitle: 'Recovering connection…' };
      case '/bms':
        return { title: 'Battery management system', subtitle: 'Recovering connection…' };
      case '/sessions/[sessionId]':
        return { title: 'Session', backHref: '#/sessions', backLabel: 'Sessions' };
      case '/sessions/boards/[savedDeviceId]':
        return { title: 'Board', backHref: '#/sessions', backLabel: 'Sessions' };
      default:
        return { title: 'Float View' };
    }
  });
  const appHeader = $derived.by(
    (): AppHeaderConfig => ({
      ...routeHeader,
      ...(headerRegistration?.owner === page.route.id ? headerRegistration.patch : {}),
    })
  );

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
        void backendState.sessionHref(sessionId).then((href) => goto(href));
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
    const preference = backendState.snapshot?.settings?.themePreference;
    if (!preference) return;
    themeController?.reconcile(preference);
  });

  $effect(() => {
    document.documentElement.classList.toggle('live-route', liveRoute);
    return () => document.documentElement.classList.remove('live-route');
  });

  $effect(() => {
    const href = connectionRedirect(
      page.route.id,
      backendState.snapshot?.activeConnection,
      backendState.status === 'connected'
    );
    if (href) void goto(href, { replaceState: true });
  });
</script>

<main
  class="bg-canvas text-foreground"
  class:min-h-screen={!liveRoute}
  class:h-dvh={liveRoute}
  class:overflow-hidden={liveRoute}
>
  <div
    class="app-safe-area mx-auto flex flex-col"
    class:min-h-screen={!liveRoute}
    class:h-full={liveRoute}
    class:min-h-0={liveRoute}
    class:max-w-3xl={!liveRoute}
    class:max-w-5xl={liveRoute}
    class:app-safe-area-with-tabs={activeBottomTab}
    class:app-safe-area-without-tabs={!activeBottomTab}
    style={`--app-header-height: ${appHeaderHeight}px`}
  >
    <PageHeader
      bind:button={appHeaderButton}
      bind:headerHeight={appHeaderHeight}
      headingId="app-title"
      title={appHeader.title}
      subtitle={appHeader.subtitle}
      subtitleSnippet={appHeader.subtitleSnippet}
      subtitleClass={appHeader.subtitleClass}
      titleClass={appHeader.titleClass}
      bottomBorder={appHeader.bottomBorder}
      backHref={appHeader.backHref}
      backLabel={appHeader.backLabel}
      menuLabel={appHeader.menu?.length ? appHeader.menuLabel : undefined}
      actions={appHeader.menu}
    />

    <div class="flex flex-1 flex-col" class:min-h-0={liveRoute}>
      {@render children()}
    </div>
  </div>
  <BottomTabBar routeId={page.route.id} />
</main>
