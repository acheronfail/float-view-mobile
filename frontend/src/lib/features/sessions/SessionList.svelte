<script lang="ts">
  import { getContext, onMount } from 'svelte';
  import { NAVIGATION_CONTEXT, type NavigationContext } from '$lib/app/navigation-context';
  import { SvelteMap, SvelteSet } from 'svelte/reactivity';
  import type { SessionListPage, SessionSummary } from '$lib/backend/contract';
  import { ApiRequestError } from '$lib/backend/client';
  import { backendState } from '$lib/backend/state.svelte';
  import { formatDistanceM } from '$lib/features/measurements/format';
  import ListItem from '$lib/ui/ListItem.svelte';
  import BoardIcon from '$lib/features/boards/BoardIcon.svelte';
  import SectionTitle from '$lib/ui/SectionTitle.svelte';
  import SessionStorageFooter from './SessionStorageFooter.svelte';
  import { sessionStartDateLabel } from './session-label';

  let { savedDeviceId }: { savedDeviceId: string } = $props();

  const PAGE_SIZE = 50;
  const navigation = getContext<NavigationContext | undefined>(NAVIGATION_CONTEXT);
  let restoreCount = 0;
  let restoringPages = $state(false);
  let disposed = false;

  function retainPages() {
    if (!navigation || loadedCatalogRevision === null) return;
    const items = groups.flatMap((group) => group.sessions.map((session) => session.summary));
    navigation.retainSessions(savedDeviceId, {
      items,
      catalogRevision: loadedCatalogRevision,
      nextCursor,
      // A return interrupted halfway through loading must retain its intended depth.
      targetCount: Math.max(items.length, restoreCount),
    });
  }
  const units = $derived(backendState.settings?.unitPreference ?? 'metric');

  type LoadedSession = {
    summary: SessionSummary;
    date: Date | null;
    dateKey: string;
    dateLabel: string;
  };

  type SessionDateGroup = {
    key: string;
    label: string;
    sessions: LoadedSession[];
  };

  let groups = $state<SessionDateGroup[]>([]);
  let nextCursor = $state<string | null>(null);
  let hasMore = $state(true);
  let loading = $state(false);
  let initialized = $state(false);
  let error = $state<string | null>(null);
  let loadedCatalogRevision = $state<number | null>(null);
  let sentinel = $state<HTMLElement | null>(null);
  let observerReady = $state(false);
  let pendingReset = $state(false);
  let requestGeneration = 0;
  let observer: IntersectionObserver | null = null;
  const groupedByDate = new SvelteMap<string, SessionDateGroup>();
  const loadedSessionIds = new SvelteSet<string>();

  function parsedDate(startedAt: string): Date | null {
    const date = new Date(startedAt);
    return Number.isNaN(date.getTime()) ? null : date;
  }

  function dateKey(date: Date | null): string {
    if (!date) return 'unknown';
    return [date.getFullYear(), date.getMonth() + 1, date.getDate()]
      .map((value) => String(value).padStart(2, '0'))
      .join('-');
  }

  function clearLoadedSessions() {
    requestGeneration += 1;
    groupedByDate.clear();
    loadedSessionIds.clear();
    groups = [];
    nextCursor = null;
    hasMore = true;
    initialized = false;
    loadedCatalogRevision = null;
  }

  function appendPage(page: SessionListPage) {
    for (const summary of page.items) {
      if (loadedSessionIds.has(summary.sessionId)) continue;
      loadedSessionIds.add(summary.sessionId);

      const date = parsedDate(summary.startedAt);
      const key = dateKey(date);
      let group = groupedByDate.get(key);
      if (!group) {
        group = {
          key,
          label: sessionStartDateLabel(summary.startedAt) ?? 'Unknown date',
          sessions: [],
        };
        groups.push(group);
        // Keep the reactive proxy: later pages can append to an existing date group.
        group = groups[groups.length - 1]!;
        groupedByDate.set(key, group);
      }
      group.sessions.push({ summary, date, dateKey: key, dateLabel: group.label });
    }
  }

  async function loadPage(reset = false, conflictRetries = 1): Promise<void> {
    if (loading) {
      if (reset) pendingReset = true;
      return;
    }
    if (!reset && !hasMore) return;

    if (reset) clearLoadedSessions();
    const generation = requestGeneration;
    const cursor = reset ? null : nextCursor;
    loading = true;
    error = null;

    try {
      const page = await backendState.listSessions(savedDeviceId, PAGE_SIZE, cursor);
      if (disposed || generation !== requestGeneration) return;

      // A state event can arrive while the page is in flight. Do not append a
      // page from the old catalog revision; restart from the first cursor.
      if (page.catalogRevision !== backendState.sessionCatalogRevision) {
        pendingReset = true;
        return;
      }

      appendPage(page);
      nextCursor = page.nextCursor;
      hasMore = page.nextCursor !== null;
      initialized = true;
      loadedCatalogRevision = page.catalogRevision;
      retainPages();
    } catch (cause) {
      if (!disposed && generation === requestGeneration) {
        if (cause instanceof ApiRequestError && cause.code === 'conflict' && conflictRetries > 0) {
          // A close, import, or expired paused ride can invalidate even the
          // first page. Restart once with a fresh cursor, then show the error.
          conflictRetries -= 1;
          pendingReset = true;
        } else error = cause instanceof Error ? cause.message : String(cause);
      }
    } finally {
      loading = false;
      if (!disposed && pendingReset) {
        pendingReset = false;
        clearLoadedSessions();
        void loadPage(true, conflictRetries);
      } else if (!disposed && restoringPages) {
        if (!error && hasMore && loadedSessionIds.size < restoreCount) void loadPage();
        else restoringPages = false;
      }
    }
  }

  $effect(() => {
    const currentRevision = backendState.sessionCatalogRevision;
    if (!initialized || loadedCatalogRevision === null || currentRevision === loadedCatalogRevision) return;
    restoreCount = Math.min(loadedSessionIds.size, navigation?.maxRetainedSessions ?? PAGE_SIZE);
    restoringPages = restoreCount > PAGE_SIZE;
    pendingReset = true;
    if (!loading) {
      pendingReset = false;
      clearLoadedSessions();
      void loadPage(true);
    }
  });

  $effect(() => {
    const element = sentinel;
    if (!observerReady || !observer || !element) return;
    observer.observe(element);
    return () => observer?.unobserve(element);
  });

  onMount(() => {
    observer =
      typeof IntersectionObserver === 'undefined'
        ? null
        : new IntersectionObserver(
            (entries) => {
              if (entries.some((entry) => entry.isIntersecting)) void loadPage();
            },
            { rootMargin: '640px 0px' }
          );
    observerReady = true;
    const retained = navigation?.sessions.get(savedDeviceId);
    if (retained?.catalogRevision === backendState.sessionCatalogRevision) {
      appendPage(retained);
      nextCursor = retained.nextCursor;
      hasMore = nextCursor !== null;
      initialized = true;
      loadedCatalogRevision = retained.catalogRevision;
      restoreCount = retained.targetCount;
      restoringPages = hasMore && loadedSessionIds.size < restoreCount;
      if (restoringPages) void loadPage();
    } else {
      restoreCount = retained?.targetCount ?? 0;
      restoringPages = restoreCount > PAGE_SIZE;
      void loadPage(true);
    }
    return () => {
      disposed = true;
      requestGeneration++;
      observer?.disconnect();
    };
  });
</script>

<section
  data-navigation-pending={(!initialized && !error) || restoringPages}
  aria-label="Sessions"
  aria-busy={loading}
  class="flex flex-col gap-4"
>
  {#if error && !groups.length}
    <p role="alert" class="text-warning">Unable to load sessions: {error}</p>
    <button class="self-start rounded bg-surface px-3 py-2 text-sm" type="button" onclick={() => loadPage(true)}>
      Try again
    </button>
  {:else if groups.length}
    {#each groups as dateGroup (dateGroup.key)}
      <div class="flex flex-col gap-2">
        <SectionTitle title={dateGroup.label} />
        <ul class="flex flex-col gap-2">
          {#each dateGroup.sessions as loaded (loaded.summary.sessionId)}
            <li>
              <ListItem
                href={`#/sessions/${encodeURIComponent(loaded.summary.sessionId)}?board=${encodeURIComponent(savedDeviceId)}`}
                title={loaded.dateLabel}
                subtitle={`${loaded.summary.state} · ${Math.round(loaded.summary.totalElapsedMs / 60000)} min · ${formatDistanceM(loaded.summary.distanceM, units)}`}
              >
                {#snippet leading()}
                  <BoardIcon
                    appearance={backendState.savedDevices.find((board) => board.savedDeviceId === savedDeviceId)
                      ?.appearance}
                    class="w-20 shrink-0"
                  />
                {/snippet}
              </ListItem>
            </li>
          {/each}
        </ul>
      </div>
    {/each}
    {#if error}
      <p role="alert" class="text-warning">Unable to load more sessions: {error}</p>
      <button class="self-start rounded bg-surface px-3 py-2 text-sm" type="button" onclick={() => loadPage()}>
        Try again
      </button>
    {/if}
  {:else if loading}
    <p class="text-muted">Loading recorded sessions…</p>
  {:else}
    <p class="text-muted">Recorded sessions will appear here.</p>
  {/if}

  {#if hasMore && !error}
    <div bind:this={sentinel} class="min-h-8" aria-label="Load more sessions">
      {#if loading}<p class="text-center text-sm text-muted">Loading more…</p>{/if}
    </div>
  {/if}
  <SessionStorageFooter {savedDeviceId} />
</section>
