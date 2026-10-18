<script lang="ts">
  import { onMount } from 'svelte';
  import type {
    ActiveConnection,
    RefloatConfigurationField,
    RefloatConfigurationSchemaPage,
    RefloatConfigurationValues,
    RefloatTune,
    RefloatTuneSummary,
  } from '$lib/backend/contract';
  import { backendState } from '$lib/backend/state.svelte';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import { formatRefloatValue, sameRefloatIdentity, sameRefloatValue } from './refloat-configuration.svelte';
  import { captureCurrentRefloatTune } from './refloat-tunes';
  import { refloatConfigurationWriteUnavailableReason } from './live';

  let { connection }: { connection: ActiveConnection } = $props();
  let tunes = $state<RefloatTuneSummary[]>([]);
  let selected = $state<RefloatTune | null>(null);
  let current = $state<RefloatConfigurationValues | null>(null);
  let fields = $state<RefloatConfigurationField[]>([]);
  let loading = $state(false);
  let switchingTuneId = $state<string | null>(null);
  let saveConfirmation = $state(false);
  let saving = $state(false);
  let saveError = $state<string | null>(null);
  let renameConfirmation = $state(false);
  let deleteConfirmation = $state(false);
  let name = $state('');
  let description = $state('');
  let status = $state<string | null>(null);

  const applyUnavailableReason = $derived(
    refloatConfigurationWriteUnavailableReason(connection, connection.refloatConfiguration, 'applyTune')
  );
  const selectedDiff = $derived.by(() => {
    if (!selected || !current) return [];
    return fields.flatMap((field) => {
      const before = current!.values[field.name];
      const after = selected!.values[field.name];
      return before && after && !sameRefloatValue(before, after) ? [{ field, before, after }] : [];
    });
  });
  const selectedIsActive = $derived(!!selected && !!current && selectedDiff.length === 0);

  onMount(() => {
    void initialize();
  });

  async function initialize() {
    loading = true;
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
    if (selected && !sameRefloatIdentity(selected.identity, loaded.identity)) selected = null;
    return loaded;
  }

  async function refreshTunes(source = current) {
    if (!source) return;
    const page = await backendState.listRefloatTunes(source.identity, 50);
    if (page) tunes = page.items;
  }

  async function selectTune(tuneId: string) {
    if (switchingTuneId || loading || backendState.busy) return;
    switchingTuneId = tuneId;
    status = null;
    try {
      const tune = await backendState.getRefloatTune(tuneId);
      if (!tune) return;
      selected = tune;
      if (!current || applyUnavailableReason || selectedIsActive) return;
      const result = await backendState.applyRefloatTune(tune.tuneId, tune.revision, current.revision);
      if (result) {
        current = result;
        status = `${tune.name} was applied and verified.`;
      }
    } finally {
      switchingTuneId = null;
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
        saveError = backendState.error ?? 'The preset could not be saved. Please try again.';
        return;
      }
      current = captured.current;
      selected = captured.tune;
      saveConfirmation = false;
      status = `${captured.tune.name} was saved from the current board Tune.`;
      await refreshTunes(captured.current);
    } catch (error) {
      saveError = error instanceof Error ? error.message : 'The preset could not be saved. Please try again.';
    } finally {
      saving = false;
    }
  }

  function openRename() {
    if (!selected) return;
    name = selected.name;
    description = selected.description ?? '';
    renameConfirmation = true;
  }

  async function rename() {
    if (!selected || !name.trim()) return;
    const updated = await backendState.updateRefloatTune(
      selected.tuneId,
      selected.revision,
      name.trim(),
      description.trim() || null,
      selected.values
    );
    if (!updated) return;
    selected = updated;
    renameConfirmation = false;
    status = 'Preset details saved.';
    await refreshTunes();
  }

  async function remove() {
    if (!selected || !(await backendState.deleteRefloatTune(selected.tuneId, selected.revision))) return;
    const deletedName = selected.name;
    selected = null;
    deleteConfirmation = false;
    status = `${deletedName} was deleted.`;
    await refreshTunes();
  }
</script>

{#if backendState.error}
  <p role="alert" class="text-warning">{backendState.error}</p>
  <button
    type="button"
    class="touch-manipulation rounded-lg bg-elevated px-4 py-2"
    onpointerdown={(event) => activateFromPointer(event, initialize)}
    onclick={(event) => activateFromKeyboardOrAssistiveClick(event, initialize)}>Retry</button
  >
{/if}
{#if status}<p role="status" class="rounded-lg border border-success/40 bg-success/10 p-3 text-sm text-success">
    {status}
  </p>{/if}

<section class="grid min-h-[65vh] content-start gap-4 lg:grid-cols-[minmax(14rem,0.7fr)_1.3fr]">
  <div class="grid content-start gap-3">
    <p class="text-sm text-muted">Save the Tune currently on the board, then tap a saved preset to switch to it.</p>
    <button
      type="button"
      class="touch-manipulation rounded-md bg-accent px-4 py-2 text-on-accent disabled:opacity-50"
      disabled={!current || loading || !!switchingTuneId || backendState.busy}
      onpointerdown={(event) => activateFromPointer(event, openSave)}
      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, openSave)}>Save current tune…</button
    >
    <div class="grid gap-2" aria-label="Saved tune presets">
      {#each tunes as tune (tune.tuneId)}
        {@const active = !!current && !!selected && selected.tuneId === tune.tuneId && selectedIsActive}
        <button
          type="button"
          aria-pressed={selected?.tuneId === tune.tuneId}
          aria-busy={switchingTuneId === tune.tuneId}
          disabled={loading || !!switchingTuneId || backendState.busy}
          class="touch-manipulation rounded-lg border bg-elevated p-3 text-left"
          class:border-accent={selected?.tuneId === tune.tuneId}
          class:border-outline={selected?.tuneId !== tune.tuneId}
          onpointerdown={(event) => activateFromPointer(event, () => selectTune(tune.tuneId), { scrollable: true })}
          onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => selectTune(tune.tuneId))}
        >
          <span class="flex items-start justify-between gap-2">
            <strong>{tune.name}</strong>
            {#if switchingTuneId === tune.tuneId}<span class="text-xs text-muted">Applying…</span>{/if}
            {#if active}<span class="rounded-full bg-success/15 px-2 py-0.5 text-xs text-success">Active</span>{/if}
          </span>
          <span class="mt-1 block text-xs text-muted"
            >{tune.description ?? `Refloat ${tune.identity.packageVersion}`}</span
          >
        </button>
      {:else}
        {#if !loading}
          <p class="rounded-lg border border-outline p-3 text-sm text-muted">No saved Tune presets yet.</p>
        {/if}
      {/each}
    </div>
  </div>

  <div class="grid content-start gap-3">
    {#if loading && !current}
      <p class="text-muted">Reading the current board Tune…</p>
    {:else if selected}
      <div>
        <p class="text-sm text-muted">Selected preset</p>
        <h3 class="text-xl font-semibold">{selected.name}</h3>
        {#if selected.description}<p class="mt-1 text-sm text-muted">{selected.description}</p>{/if}
      </div>
      <div class="flex flex-wrap gap-2">
        <button
          type="button"
          class="touch-manipulation rounded-md border border-outline px-4 py-2 disabled:opacity-50"
          disabled={!!switchingTuneId || backendState.busy}
          onpointerdown={(event) => activateFromPointer(event, openRename)}
          onclick={(event) => activateFromKeyboardOrAssistiveClick(event, openRename)}>Rename</button
        >
        <button
          type="button"
          class="touch-manipulation rounded-md border border-danger px-4 py-2 text-danger disabled:opacity-50"
          disabled={!!switchingTuneId || backendState.busy}
          onpointerdown={(event) =>
            activateFromPointer(event, () => {
              deleteConfirmation = true;
            })}
          onclick={(event) =>
            activateFromKeyboardOrAssistiveClick(event, () => {
              deleteConfirmation = true;
            })}>Delete</button
        >
      </div>
      {#if selectedIsActive}
        <p role="status" class="rounded-lg border border-success/40 bg-success/10 p-3 text-sm text-success">
          This preset matches the Tune currently on the board.
        </p>
      {:else if applyUnavailableReason}
        <p role="status" class="rounded-lg border border-warning/40 bg-warning/10 p-3 text-sm text-warning">
          {applyUnavailableReason}
        </p>
      {/if}
      <div class="rounded-lg border border-outline p-3 text-sm">
        <strong>{selectedDiff.length} values differ from the board</strong>
        {#if selectedDiff.length}
          <ul class="mt-2 max-h-72 overflow-y-auto">
            {#each selectedDiff as item (item.field.name)}
              <li>
                {item.field.label}: {formatRefloatValue(item.before, item.field)} → {formatRefloatValue(
                  item.after,
                  item.field
                )}
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {:else}
      <p class="rounded-lg border border-outline p-4 text-sm text-muted">
        Tap a saved preset to apply it immediately. Select it to see its details when switching is unavailable.
      </p>
    {/if}
  </div>
</section>

<ConfirmDialog
  open={saveConfirmation}
  titleId="refloat-tune-save-title"
  title="Save current board Tune?"
  description="The board will be read again, then its complete current Tune will be saved as a preset."
  confirmLabel="Save preset"
  busy={saving || backendState.busy}
  confirmDisabled={!name.trim()}
  onconfirm={saveCurrent}
  oncancel={() => {
    saveConfirmation = false;
  }}
>
  <div class="grid gap-3">
    {#if saveError}<p role="alert" class="text-sm text-warning">{saveError}</p>{/if}
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
  title="Rename preset"
  description="This changes only the preset name and description, not its saved Tune values."
  confirmLabel="Save details"
  busy={backendState.busy}
  confirmDisabled={!name.trim()}
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
  title="Delete {selected?.name ?? 'preset'}?"
  description="This removes the saved preset from this app. The board is not changed."
  confirmLabel="Delete preset"
  appearance="destructive"
  busy={backendState.busy}
  onconfirm={remove}
  oncancel={() => {
    deleteConfirmation = false;
  }}
/>
