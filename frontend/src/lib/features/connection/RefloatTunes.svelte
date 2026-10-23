<script lang="ts">
  import { onMount } from 'svelte';
  import type {
    ActiveConnection,
    RefloatConfigurationField,
    RefloatConfigurationIdentity,
    RefloatConfigurationSchemaPage,
    RefloatConfigurationValues,
    RefloatTune,
    RefloatTuneSummary,
  } from '$lib/backend/contract';
  import { backendState } from '$lib/backend/state.svelte';
  import InformationDialog from '$lib/ui/InformationDialog.svelte';
  import Menu from '$lib/ui/Menu.svelte';
  import MenuItem from '$lib/ui/MenuItem.svelte';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import { sameRefloatIdentity, sameRefloatValue } from './refloat-configuration.svelte';
  import { captureCurrentRefloatTune, overwriteCurrentRefloatTune } from './refloat-tunes';
  import { refloatConfigurationWriteUnavailableReason } from './live';

  let { connection }: { connection: ActiveConnection } = $props();
  let tunes = $state<RefloatTuneSummary[]>([]);
  let current = $state<RefloatConfigurationValues | null>(null);
  let matchingCursor = $state<string | null>(null);
  let matchingComplete = $state(false);
  let libraryComplete = $state(false);
  let loadMoreTarget = $state<HTMLElement>();
  let managing = $state(false);
  let managedTune = $state<RefloatTune | null>(null);
  let information = $state<string | null>(null);
  let informationTrigger = $state<HTMLElement>();
  let listLoading = $state(false);
  let nextCursor = $state<string | null>(null);
  let listLoaded = $state(false);
  const packageVersion = $derived(current?.identity.packageVersion ?? connection.refloatConfiguration?.packageVersion);
  function incompatibility(identity: RefloatConfigurationIdentity): string | null {
    if (!current) return 'Read the board tune before applying.';
    if (identity.packageVersion !== current.identity.packageVersion) {
      return `Requires Refloat ${identity.packageVersion}. This board uses ${current.identity.packageVersion}.`;
    }
    if (!sameRefloatIdentity(identity, current.identity)) {
      return `This tune uses a different configuration and cannot be applied to this board.`;
    }
    return null;
  }
  const sortedTunes = $derived(
    [...tunes].sort((a, b) => {
      const rank = (tune: RefloatTuneSummary) =>
        !incompatibility(tune.identity) ? 0 : tune.identity.packageVersion === packageVersion ? 1 : 2;
      return rank(a) - rank(b);
    })
  );
  const hasMore = $derived(!matchingComplete || !libraryComplete);
  $effect(() => {
    if (!loadMoreTarget || loading || listLoading || backendState.error || !hasMore || tunes.length >= 1000) return;
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) void loadTunes();
    });
    observer.observe(loadMoreTarget);
    return () => observer.disconnect();
  });
  let selected = $state<RefloatTune | null>(null);
  let fields = $state<RefloatConfigurationField[]>([]);
  let loading = $state(false);
  let switchError = $state<{ tuneId: string; message: string } | null>(null);
  let switchingTuneId = $state<string | null>(null);
  let pendingApply = $state<{ summary: RefloatTuneSummary; trigger: HTMLElement } | null>(null);
  let saveConfirmation = $state(false);
  let saving = $state(false);
  let overwriteConfirmation = $state(false);
  let overwriting = $state(false);
  let overwriteError = $state<string | null>(null);
  let saveError = $state<string | null>(null);
  let renameError = $state<string | null>(null);
  let deleteError = $state<string | null>(null);
  let renameConfirmation = $state(false);
  let deleteConfirmation = $state(false);
  let name = $state('');
  let description = $state('');
  let status = $state<string | null>(null);

  const applyUnavailableReason = $derived(
    refloatConfigurationWriteUnavailableReason(connection, connection.refloatConfiguration, 'applyTune')
  );
  const selectedIsActive = $derived(
    !!selected &&
      !!current &&
      !incompatibility(selected.identity) &&
      fields.every((field) => {
        const saved = selected!.values[field.name];
        const actual = current!.values[field.name];
        return !!saved && !!actual && sameRefloatValue(saved, actual);
      })
  );

  onMount(() => {
    void initialize();
  });

  async function initialize() {
    loading = true;
    switchError = null;
    status = null;
    try {
      if (!(await backendState.openRefloatConfiguration())) return;
      const pages: RefloatConfigurationSchemaPage[] = [];
      let cursor: string | null = null;
      for (let count = 0; count < 64; count += 1) {
        const page = await backendState.refloatConfigurationSchema(cursor);
        if (!page) return;
        pages.push(page);
        cursor = page.nextCursor;
        if (!cursor) break;
      }
      if (cursor || !pages.length) return;
      fields = pages.flatMap((page) => page.fields).filter((field) => field.tunable);
      const loaded = await readCurrent(true);
      if (!loaded) return;
      await refreshTunes(loaded);
    } finally {
      loading = false;
    }
  }

  async function readCurrent(refresh: boolean) {
    const loaded = await backendState.readRefloatConfiguration('current', refresh);
    if (!loaded) return null;
    current = loaded;
    return loaded;
  }

  async function refreshTunes(source = current) {
    if (!source) return;
    matchingCursor = null;
    nextCursor = null;
    matchingComplete = false;
    libraryComplete = false;
    tunes = [];
    await loadTunes(source);
  }

  async function loadTunes(source = current) {
    if (!source || listLoading) return;
    listLoading = true;
    try {
      if (!matchingComplete) {
        const matching = await backendState.listRefloatTunes(source.identity, 50, matchingCursor);
        if (!matching) return;
        tunes = [...tunes, ...matching.items].slice(0, 1000);
        matchingCursor = matching.nextCursor;
        matchingComplete = !matchingCursor;
      }
      if (matchingComplete && !libraryComplete && tunes.length < 1000) {
        const page = await backendState.listRefloatTunes(null, 50, nextCursor);
        if (!page) return;
        const existing = new Set(tunes.map((tune) => tune.tuneId));
        tunes = [...tunes, ...page.items.filter((tune) => !existing.has(tune.tuneId))].slice(0, 1000);
        nextCursor = page.nextCursor;
        libraryComplete = !nextCursor;
      }
      listLoaded = true;
    } finally {
      listLoading = false;
    }
  }

  async function selectTune(summary: RefloatTuneSummary, trigger: HTMLElement, confirmed = false) {
    if (switchingTuneId || managing || loading || listLoading || backendState.busy) return;
    const reason = incompatibility(summary.identity) ?? applyUnavailableReason;
    if (reason) {
      informationTrigger = trigger;
      information = reason;
      return;
    }
    switchingTuneId = summary.tuneId;
    switchError = null;
    status = null;
    try {
      const tune = await backendState.getRefloatTune(summary.tuneId);
      if (!tune) {
        switchError = { tuneId: summary.tuneId, message: backendState.error ?? 'Could not load tune. Try again.' };
        return;
      }
      selected = tune;
      if (!current || selectedIsActive) return;
      if (!confirmed && connection.latestSample?.rideState !== 'idle') {
        pendingApply = { summary, trigger };
        return;
      }
      const result = await backendState.applyRefloatTune(tune.tuneId, tune.revision, current.revision);
      if (result) {
        current = result;
        status = `${tune.name} applied.`;
      } else {
        switchError = { tuneId: tune.tuneId, message: backendState.error ?? 'Could not apply tune. Try again.' };
      }
    } finally {
      switchingTuneId = null;
    }
  }

  async function manageTune(tune: RefloatTuneSummary, action: 'rename' | 'delete' | 'overwrite') {
    if (loading || listLoading || switchingTuneId || managing || backendState.busy) return;
    managing = true;
    try {
      managedTune = await backendState.getRefloatTune(tune.tuneId);
      if (!managedTune) return;
      if (action === 'rename') openRename();
      else if (action === 'overwrite') {
        if (incompatibility(managedTune.identity)) return;
        overwriteError = null;
        overwriteConfirmation = true;
      } else {
        deleteError = null;
        deleteConfirmation = true;
      }
    } finally {
      managing = false;
    }
  }

  function openSave() {
    saveError = null;
    name = '';
    description = '';
    saveConfirmation = true;
  }

  async function saveCurrent() {
    if (!name.trim() || saving) return;
    saving = true;
    saveError = null;
    try {
      const captured = await captureCurrentRefloatTune(
        backendState,
        name.trim(),
        description.trim() || null,
        fields.map((field) => field.name)
      );
      if (!captured) {
        saveError = backendState.error ?? 'Could not save tune. Try again.';
        return;
      }
      current = captured.current;
      selected = captured.tune;
      saveConfirmation = false;
      status = `${captured.tune.name} saved.`;
      await refreshTunes(captured.current);
    } catch (error) {
      saveError = error instanceof Error ? error.message : 'Could not save tune. Try again.';
    } finally {
      saving = false;
    }
  }

  async function overwrite() {
    if (!managedTune || overwriting) return;
    overwriting = true;
    overwriteError = null;
    try {
      const captured = await overwriteCurrentRefloatTune(
        backendState,
        managedTune,
        fields.map((field) => field.name)
      );
      if (!captured) {
        overwriteError = backendState.error ?? 'Could not overwrite tune. Try again.';
        return;
      }
      current = captured.current;
      selected = captured.tune;
      managedTune = captured.tune;
      overwriteConfirmation = false;
      status = `${captured.tune.name} saved.`;
      await refreshTunes(captured.current);
    } catch (error) {
      overwriteError = error instanceof Error ? error.message : 'Could not overwrite tune. Try again.';
    } finally {
      overwriting = false;
    }
  }

  function openRename() {
    if (!managedTune) return;
    name = managedTune.name;
    description = managedTune.description ?? '';
    renameError = null;
    renameConfirmation = true;
  }

  async function rename() {
    if (!managedTune || !name.trim()) return;
    renameError = null;
    const updated = await backendState.updateRefloatTune(
      managedTune.tuneId,
      managedTune.revision,
      name.trim(),
      description.trim() || null,
      managedTune.values
    );
    if (!updated) {
      renameError = backendState.error ?? 'Could not rename tune. Try again.';
      return;
    }
    if (selected?.tuneId === updated.tuneId) selected = updated;
    managedTune = updated;
    renameConfirmation = false;
    status = 'Changes saved.';
    await refreshTunes();
  }

  async function remove() {
    if (!managedTune) return;
    deleteError = null;
    if (!(await backendState.deleteRefloatTune(managedTune.tuneId, managedTune.revision))) {
      deleteError = backendState.error ?? 'Could not delete tune. Try again.';
      return;
    }
    const deletedName = managedTune.name;
    if (selected?.tuneId === managedTune.tuneId) selected = null;
    managedTune = null;
    deleteConfirmation = false;
    status = `${deletedName} deleted.`;
    await refreshTunes();
  }
</script>

{#if backendState.error && !switchError && !saveConfirmation && !renameConfirmation && !deleteConfirmation && !overwriteConfirmation}
  <p role="alert" class="text-warning">{backendState.error}</p>
  <button
    type="button"
    class="touch-manipulation rounded-lg bg-elevated px-4 py-2"
    onpointerdown={(event) => activateFromPointer(event, initialize)}
    onclick={(event) => activateFromKeyboardOrAssistiveClick(event, initialize)}>Retry</button
  >
{/if}
{#if status}<p role="status" class="mb-3 rounded-lg border border-success/40 bg-success/10 p-3 text-sm text-success">
    {status}
  </p>{/if}

<section class="grid content-start gap-3">
  <button
    type="button"
    class="touch-manipulation rounded-md bg-accent-fill px-4 py-2 text-on-accent disabled:opacity-50"
    disabled={!current || loading || listLoading || !!switchingTuneId || managing || backendState.busy}
    onpointerdown={(event) => activateFromPointer(event, openSave)}
    onclick={(event) => activateFromKeyboardOrAssistiveClick(event, openSave)}>Save tune</button
  >
  <div class="grid gap-3" aria-label="Saved tunes">
    {#each sortedTunes as tune (tune.tuneId)}
      {@const reason = incompatibility(tune.identity)}
      {@const active = selected?.tuneId === tune.tuneId && selectedIsActive}
      <div>
        <div
          class="flex rounded-xl border bg-elevated"
          class:border-accent={active}
          class:border-outline={!active}
          data-tune-id={tune.tuneId}
          data-compatible={!reason}
        >
          <button
            type="button"
            aria-busy={switchingTuneId === tune.tuneId}
            aria-describedby={reason ? `tune-${tune.tuneId}-compatibility` : undefined}
            disabled={loading || listLoading || !!switchingTuneId || managing || backendState.busy}
            class="min-w-0 flex-1 touch-pan-y rounded-l-xl p-4 text-left disabled:opacity-50"
            class:text-muted={!!reason}
            class:opacity-60={!!reason}
            aria-haspopup={reason || applyUnavailableReason || connection.latestSample?.rideState !== 'idle'
              ? 'dialog'
              : undefined}
            onpointerdown={(event) => {
              const trigger = event.currentTarget;
              activateFromPointer(event, () => selectTune(tune, trigger), { scrollable: true });
            }}
            onclick={(event) => {
              const trigger = event.currentTarget;
              activateFromKeyboardOrAssistiveClick(event, () => selectTune(tune, trigger));
            }}
          >
            <span class="flex flex-wrap items-center justify-between gap-2">
              <strong class="break-words">{tune.name}</strong>
              <span
                class="rounded-full border px-2 py-0.5 text-xs whitespace-nowrap"
                class:border-outline={!!reason}
                class:text-muted={!!reason}
                class:border-accent={!reason}
                class:text-accent={!reason}>Refloat {tune.identity.packageVersion}</span
              >
            </span>
            {#if tune.description}<span class="mt-1 block text-sm">{tune.description}</span>{/if}
            <span class="mt-2 block text-xs" id={`tune-${tune.tuneId}-compatibility`}>
              {#if switchingTuneId === tune.tuneId}Applying…{:else if active}Active{:else if reason}Unavailable{:else}Tap
                to apply{/if}
            </span>
          </button>
          <Menu
            label={`Options for ${tune.name}`}
            variant="card"
            disabled={loading || listLoading || !!switchingTuneId || managing || backendState.busy}
          >
            {#snippet children(closeMenu)}
              <MenuItem
                label="Overwrite"
                disabled={!!reason || overwriting}
                onactivate={() => {
                  closeMenu();
                  return manageTune(tune, 'overwrite');
                }}
              />
              <MenuItem
                label="Rename"
                onactivate={() => {
                  closeMenu();
                  return manageTune(tune, 'rename');
                }}
              />
              <MenuItem
                label="Delete"
                appearance="destructive"
                onactivate={() => {
                  closeMenu();
                  return manageTune(tune, 'delete');
                }}
              />
            {/snippet}
          </Menu>
        </div>
        {#if switchError?.tuneId === tune.tuneId}<p role="alert" class="mt-2 text-sm text-warning">
            {switchError.message}
          </p>{/if}
      </div>
    {:else}
      {#if !loading && listLoaded}<p class="py-4 text-center text-sm text-muted">No saved tunes.</p>{/if}
    {/each}
    {#if loading || listLoading}<p role="status" class="text-sm text-muted">Loading…</p>{/if}
    {#if hasMore && tunes.length < 1000 && !backendState.error}<div
        bind:this={loadMoreTarget}
        class="h-1"
        aria-hidden="true"
      ></div>
    {:else if hasMore && tunes.length >= 1000}<p class="text-sm text-muted">Showing the first 1,000 tunes.</p>{/if}
  </div>
</section>

<InformationDialog
  open={information !== null}
  titleId="tune-unavailable-title"
  title="Tune unavailable"
  description={information ?? ''}
  returnFocus={informationTrigger}
  onclose={() => {
    information = null;
  }}
/>

<ConfirmDialog
  open={pendingApply !== null}
  titleId="refloat-tune-apply-title"
  title="Apply {pendingApply?.summary.name ?? 'tune'}?"
  description="The board is not idle. Applying a tune can cause the board to cut out momentarily. Continue?"
  confirmLabel="Apply tune"
  appearance="warning"
  confirmDisabled={!!applyUnavailableReason}
  error={applyUnavailableReason}
  onconfirm={() => {
    const pending = pendingApply;
    if (!pending) return;
    pendingApply = null;
    return selectTune(pending.summary, pending.trigger, true);
  }}
  oncancel={() => {
    pendingApply = null;
  }}
/>

<ConfirmDialog
  open={saveConfirmation}
  titleId="refloat-tune-save-title"
  title="Save current tune"
  confirmLabel="Save tune"
  busy={saving || backendState.busy}
  confirmDisabled={!name.trim()}
  error={saveError}
  onconfirm={saveCurrent}
  oncancel={() => {
    saveConfirmation = false;
  }}
>
  <div class="grid gap-3">
    <label class="text-sm">
      <span class="mb-1 block text-muted">Name</span>
      <input class="w-full rounded-md border border-outline bg-elevated px-3 py-2" maxlength="128" bind:value={name} />
    </label>
    <label class="text-sm">
      <span class="mb-1 block text-muted">Description (optional)</span>
      <textarea
        class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
        maxlength="1024"
        rows="3"
        bind:value={description}
      ></textarea>
    </label>
  </div>
</ConfirmDialog>

<ConfirmDialog
  open={renameConfirmation}
  titleId="refloat-tune-rename-title"
  title="Rename tune"
  confirmLabel="Save"
  busy={backendState.busy}
  confirmDisabled={!name.trim()}
  error={renameError}
  onconfirm={rename}
  oncancel={() => {
    renameConfirmation = false;
  }}
>
  <div class="grid gap-3">
    <label class="text-sm">
      <span class="mb-1 block text-muted">Name</span>
      <input class="w-full rounded-md border border-outline bg-elevated px-3 py-2" maxlength="128" bind:value={name} />
    </label>
    <label class="text-sm">
      <span class="mb-1 block text-muted">Description (optional)</span>
      <textarea
        class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
        maxlength="1024"
        rows="3"
        bind:value={description}
      ></textarea>
    </label>
  </div>
</ConfirmDialog>

<ConfirmDialog
  open={deleteConfirmation}
  titleId="refloat-tune-delete-title"
  title="Delete {managedTune?.name ?? 'tune'}?"
  description="The board’s tune will stay unchanged."
  confirmLabel="Delete tune"
  appearance="destructive"
  busy={backendState.busy}
  error={deleteError}
  onconfirm={remove}
  oncancel={() => {
    deleteConfirmation = false;
  }}
/>

<ConfirmDialog
  open={overwriteConfirmation}
  titleId="refloat-tune-overwrite-title"
  title="Overwrite {managedTune?.name ?? 'tune'}?"
  description="Replace this saved tune with the board’s current tune."
  confirmLabel="Overwrite"
  busy={overwriting || backendState.busy}
  error={overwriteError}
  onconfirm={overwrite}
  oncancel={() => {
    overwriteConfirmation = false;
  }}
/>
