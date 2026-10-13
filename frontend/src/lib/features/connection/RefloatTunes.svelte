<script lang="ts">
  import { onMount } from 'svelte';
  import type { RefloatTune, RefloatTuneSummary } from '$lib/backend/contract';
  import { backendState } from '$lib/backend/state.svelte';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import {
    formatRefloatValue,
    sameRefloatValue,
    type RefloatConfigurationController,
  } from './refloat-configuration.svelte';

  let { controller }: { controller: RefloatConfigurationController } = $props();
  let tunes = $state<RefloatTuneSummary[]>([]);
  let selected = $state<RefloatTune | null>(null);
  let name = $state('');
  let description = $state('');
  let applyConfirmation = $state(false);
  let deleteConfirmation = $state(false);

  const selectedDiff = $derived.by(() => {
    if (!selected || !controller.loaded) return [];
    return controller.fields.flatMap((field) => {
      const before = controller.loaded!.values[field.name];
      const after = selected!.values[field.name];
      return before && after && !sameRefloatValue(before, after) ? [{ field, before, after }] : [];
    });
  });

  onMount(() => {
    void initialize();
  });

  async function initialize() {
    if (!controller.identity || controller.error) await controller.open();
    await refresh();
  }

  async function refresh() {
    if (!controller.identity) return;
    const page = await backendState.listRefloatTunes(controller.identity, 50);
    if (page) tunes = page.items;
  }

  async function create() {
    if (!controller.identity || !controller.loaded || !name.trim()) return;
    const created = await backendState.createRefloatTune(
      name.trim(),
      description.trim() || null,
      controller.identity,
      controller.loaded.revision,
      controller.draft
    );
    if (!created) return;
    name = '';
    description = '';
    selected = created;
    await refresh();
  }

  async function selectTune(tuneId: string) {
    const tune = await backendState.getRefloatTune(tuneId);
    if (!tune) return;
    selected = tune;
    name = tune.name;
    description = tune.description ?? '';
  }

  async function update() {
    if (!selected || !name.trim()) return;
    const updated = await backendState.updateRefloatTune(
      selected.tuneId,
      selected.revision,
      name.trim(),
      description.trim() || null,
      controller.draft
    );
    if (!updated) {
      const current = await backendState.getRefloatTune(selected.tuneId);
      if (current) {
        selected = current;
        name = current.name;
        description = current.description ?? '';
      }
      return;
    }
    selected = updated;
    await refresh();
  }

  async function apply() {
    if (!selected || !controller.loaded) return;
    const result = await backendState.applyRefloatTune(selected.tuneId, selected.revision, controller.loaded.revision);
    if (result) {
      controller.acceptValues(result);
      applyConfirmation = false;
    }
  }

  async function remove() {
    if (!selected || !(await backendState.deleteRefloatTune(selected.tuneId, selected.revision))) return;
    selected = null;
    deleteConfirmation = false;
    await refresh();
  }
</script>

{#if controller.error || backendState.error}
  <p role="alert" class="text-warning">{controller.error || backendState.error}</p>
  <button
    type="button"
    class="touch-manipulation rounded-lg bg-elevated px-4 py-2"
    onpointerdown={(event) => activateFromPointer(event, initialize)}
    onclick={(event) => activateFromKeyboardOrAssistiveClick(event, initialize)}>Retry</button
  >
{/if}
<section class="grid min-h-[65vh] gap-4 lg:grid-cols-[minmax(14rem,0.7fr)_1.3fr]">
  <div>
    <p class="mb-3 text-sm text-muted">
      Tunes are complete exact-version Refloat configurations. They are not upstream QML curated Tunes and are not
      migrated between versions.
    </p>
    <div class="grid gap-2">
      {#each tunes as tune (tune.tuneId)}
        <button
          type="button"
          class="touch-manipulation rounded-lg border border-outline bg-elevated p-3 text-left"
          onpointerdown={(event) => activateFromPointer(event, () => selectTune(tune.tuneId))}
          onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => selectTune(tune.tuneId))}
        >
          <strong class="block">{tune.name}</strong>
          <span class="text-xs text-muted">{tune.description ?? `Refloat ${tune.identity.packageVersion}`}</span>
        </button>
      {:else}
        <p class="text-sm text-muted">No compatible Tunes saved.</p>
      {/each}
    </div>
  </div>

  <div class="grid content-start gap-3">
    <h3 class="text-lg font-semibold">{selected ? 'Edit Tune' : 'Save complete configuration'}</h3>
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
    <div class="flex flex-wrap gap-2">
      <button
        type="button"
        class="touch-manipulation rounded-md bg-accent px-4 py-2 text-on-accent disabled:opacity-50"
        disabled={!name.trim() || !controller.loaded || backendState.busy}
        onpointerdown={(event) => activateFromPointer(event, selected ? update : create)}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, selected ? update : create)}
        >{selected ? 'Update from draft' : 'Save draft as Tune'}</button
      >
      {#if selected}
        <button
          type="button"
          class="touch-manipulation rounded-md border border-outline px-4 py-2 disabled:opacity-50"
          disabled={!controller.summary?.writable || !controller.loaded || backendState.busy}
          onpointerdown={(event) =>
            activateFromPointer(event, () => {
              applyConfirmation = true;
            })}
          onclick={(event) =>
            activateFromKeyboardOrAssistiveClick(event, () => {
              applyConfirmation = true;
            })}>Apply and verify</button
        >
        <button
          type="button"
          class="touch-manipulation rounded-md border border-danger px-4 py-2 text-danger"
          onpointerdown={(event) =>
            activateFromPointer(event, () => {
              deleteConfirmation = true;
            })}
          onclick={(event) =>
            activateFromKeyboardOrAssistiveClick(event, () => {
              deleteConfirmation = true;
            })}>Delete</button
        >
      {/if}
    </div>
    {#if selected}
      <div class="rounded-lg border border-outline p-3 text-sm">
        <strong>{selectedDiff.length} values differ from current</strong>
        <ul class="mt-2 max-h-56 overflow-y-auto">
          {#each selectedDiff as item (item.field.name)}
            <li>
              {item.field.label}: {formatRefloatValue(item.before, item.field)} → {formatRefloatValue(
                item.after,
                item.field
              )}
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </div>
</section>

<ConfirmDialog
  open={applyConfirmation}
  titleId="refloat-tune-apply-title"
  title="Apply complete Refloat configuration?"
  description="The full Tune will be written persistently and every value verified by readback."
  confirmLabel="Apply and verify"
  appearance="warning"
  busy={backendState.busy}
  confirmDisabled={!controller.summary?.writable || !controller.loaded}
  onconfirm={apply}
  oncancel={() => {
    applyConfirmation = false;
  }}
/>

<ConfirmDialog
  open={deleteConfirmation}
  titleId="refloat-tune-delete-title"
  title="Delete Tune?"
  description="This removes the saved complete configuration from this app."
  confirmLabel="Delete"
  appearance="destructive"
  busy={backendState.busy}
  onconfirm={remove}
  oncancel={() => {
    deleteConfirmation = false;
  }}
/>
