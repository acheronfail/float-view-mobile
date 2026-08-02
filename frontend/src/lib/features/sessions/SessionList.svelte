<script lang="ts">
  import { onMount } from 'svelte';
  import { SvelteMap, SvelteSet } from 'svelte/reactivity';
  import type { SessionListPage, SessionSummary } from '$lib/backend/contract';
  import { backendState } from '$lib/backend/state.svelte';
  import { formatDistanceM } from '$lib/features/measurements/format';
  import ListItem from '$lib/ui/ListItem.svelte';
  import SectionTitle from '$lib/ui/SectionTitle.svelte';

  const PAGE_SIZE = 50;
  const units = $derived(backendState.settings?.unitPreference ?? 'metric');
  const dateFormatter = new Intl.DateTimeFormat(undefined, { year: 'numeric', month: 'long', day: 'numeric' });

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
          label: date ? dateFormatter.format(date) : 'Unknown date',
          sessions: [],
        };
        groupedByDate.set(key, group);
        groups.push(group);
      }
      group.sessions.push({ summary, date, dateKey: key, dateLabel: group.label });
    }
  }

  async function loadPage(reset = false): Promise<void> {
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
      const page = await backendState.listSessions(PAGE_SIZE, cursor);
      if (generation !== requestGeneration) return;

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
    } catch (cause) {
      if (generation === requestGeneration) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    } finally {
      loading = false;
      if (pendingReset) {
        pendingReset = false;
        clearLoadedSessions();
        void loadPage(true);
      }
    }
  }

  $effect(() => {
    const currentRevision = backendState.sessionCatalogRevision;
    if (!initialized || loadedCatalogRevision === null || currentRevision === loadedCatalogRevision) return;
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
    void loadPage(true);
    return () => observer?.disconnect();
  });
</script>

<section aria-label="Sessions" aria-busy={loading} class="flex flex-col gap-4">
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
                href={`#/sessions/${encodeURIComponent(loaded.summary.sessionId)}`}
                title={loaded.summary.displayName}
                subtitle={`${loaded.summary.state} · ${Math.round(loaded.summary.totalElapsedMs / 60000)} min · ${formatDistanceM(loaded.summary.distanceM, units)}`}
              />
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
</section>
