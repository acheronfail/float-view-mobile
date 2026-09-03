<script lang="ts">
  import { onMount } from 'svelte';
  import type { RefloatConfigurationField as RefloatConfigurationFieldContract } from '$lib/backend/contract';
  import { backendState } from '$lib/backend/state.svelte';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import InformationDialog from '$lib/ui/InformationDialog.svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import RefloatConfigurationField from './RefloatConfigurationField.svelte';
  import {
    formatRefloatValue,
    refloatConfigurationFieldHelp,
    refloatConfigurationFieldLabel,
    resolveRefloatConfigurationSection,
    type RefloatConfigurationController,
  } from './refloat-configuration.svelte';

  let { controller }: { controller: RefloatConfigurationController } = $props();
  let writeConfirmation = $state(false);
  let selectedSectionKey = $state('');
  let helpField = $state<RefloatConfigurationFieldContract | null>(null);
  let helpTrigger = $state<HTMLButtonElement>();
  const selectedSection = $derived(resolveRefloatConfigurationSection(controller.sections, selectedSectionKey));
  const visibleFields = $derived(controller.filteredSectionFields(selectedSection));

  $effect(() => {
    const sections = controller.sections;
    if (!sections.some((section) => section.key === selectedSectionKey)) {
      selectedSectionKey = sections[0]?.key ?? '';
    }
  });

  onMount(() => {
    void controller.open();
  });

  export function hasUnsavedChanges(): boolean {
    return controller.diff.length > 0;
  }

  async function write() {
    if (await controller.write()) writeConfirmation = false;
  }

  function showHelp(field: RefloatConfigurationFieldContract, trigger: HTMLButtonElement) {
    helpField = field;
    helpTrigger = trigger;
  }

  function closeHelp() {
    helpField = null;
  }
</script>

<section class="flex min-h-[70vh] flex-col gap-3" aria-labelledby="refloat-configuration-title">
  <div class="sticky top-0 z-10 grid gap-2 bg-surface pb-2">
    <div class="grid gap-2 sm:grid-cols-2">
      <label class="grid gap-1 text-sm">
        <span class="font-medium">Section</span>
        <select
          class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
          disabled={controller.loading || !controller.sections.length}
          value={selectedSection?.key ?? ''}
          onchange={(event) => {
            selectedSectionKey = event.currentTarget.value;
          }}
        >
          {#each controller.sections as section (section.key)}
            <option value={section.key}>{section.label}</option>
          {/each}
        </select>
      </label>
      <label class="grid gap-1 text-sm">
        <span class="font-medium">Search this section</span>
        <input
          type="search"
          placeholder="Search fields"
          class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
          bind:value={controller.search}
        />
      </label>
    </div>
    <div class="flex flex-wrap justify-end gap-2">
      <button
        type="button"
        class="touch-manipulation rounded-md border border-outline px-3 py-2 disabled:opacity-50"
        disabled={controller.loading || backendState.busy || controller.summary?.pendingOperation !== null}
        onpointerdown={(event) => activateFromPointer(event, () => controller.load('current'))}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => controller.load('current'))}
        >Read current</button
      >
      <button
        type="button"
        class="touch-manipulation rounded-md border border-outline px-3 py-2 disabled:opacity-50"
        disabled={controller.loading || backendState.busy || controller.summary?.pendingOperation !== null}
        onpointerdown={(event) => activateFromPointer(event, () => controller.load('default'))}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => controller.load('default'))}
        >Read defaults</button
      >
    </div>
  </div>

  {#if controller.loading && !controller.pages.length}
    <p class="text-muted">Loading audited Refloat schema…</p>
  {:else if controller.error}
    <p role="alert" class="text-warning">{controller.error}</p>
  {:else}
    <div class="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-outline p-3 text-sm">
      <span>
        Source: <strong
          >{controller.source === 'current' ? 'Current device values' : 'Persistent firmware defaults'}</strong
        >
      </span>
      <span>{controller.diff.length} changed</span>
    </div>
    {#if controller.summary?.writable === false}
      <p class="rounded-lg border border-warning/40 bg-warning/10 p-3 text-sm text-warning">
        Persistent writes are unavailable. The board may be read-only, reconnecting, unsafe, or in flywheel mode.
      </p>
    {/if}
    {#if selectedSection}
      <section class="grid gap-2" aria-labelledby="refloat-selected-section">
        <h3 id="refloat-selected-section" class="text-lg font-semibold">
          {selectedSection.group} · {selectedSection.subgroup}
        </h3>
        {#if visibleFields.length}
          <div class="grid gap-2 md:grid-cols-2">
            {#each visibleFields as field (field.name)}
              <RefloatConfigurationField
                {field}
                value={controller.draft[field.name]}
                error={controller.validationErrors[field.name]}
                disabled={controller.loading || backendState.busy}
                onchange={(value) => controller.setValue(field.name, value)}
                onhelp={showHelp}
              />
            {/each}
          </div>
        {:else}
          <p class="rounded-lg border border-outline p-3 text-sm text-muted">No fields match this search.</p>
        {/if}
      </section>
    {/if}
    <div class="sticky bottom-0 flex items-center justify-end gap-2 border-t border-outline bg-surface py-3">
      <button
        type="button"
        class="touch-manipulation rounded-md border border-outline px-4 py-2"
        disabled={!controller.diff.length || controller.loading}
        onpointerdown={(event) => activateFromPointer(event, () => controller.resetDraft())}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => controller.resetDraft())}
        >Discard changes</button
      >
      <button
        type="button"
        class="touch-manipulation rounded-md bg-accent px-4 py-2 text-on-accent disabled:opacity-50"
        disabled={!controller.canWrite || controller.loading || backendState.busy}
        onpointerdown={(event) =>
          activateFromPointer(event, () => {
            writeConfirmation = true;
          })}
        onclick={(event) =>
          activateFromKeyboardOrAssistiveClick(event, () => {
            writeConfirmation = true;
          })}>Write complete configuration</button
      >
    </div>
  {/if}
</section>

<InformationDialog
  open={helpField !== null}
  titleId="refloat-field-help-title"
  title={helpField ? refloatConfigurationFieldLabel(helpField) : ''}
  description={helpField ? refloatConfigurationFieldHelp(helpField) : ''}
  returnFocus={helpTrigger}
  onclose={closeHelp}
/>

<ConfirmDialog
  open={writeConfirmation}
  titleId="refloat-write-confirmation-title"
  title="Write persistent Refloat configuration?"
  description="This writes one complete configuration and verifies every value by reading it back."
  confirmLabel="Write and verify"
  appearance="warning"
  busy={controller.loading || backendState.busy}
  confirmDisabled={!controller.canWrite}
  onconfirm={write}
  oncancel={() => {
    writeConfirmation = false;
  }}
>
  <ul class="mt-3 max-h-48 overflow-y-auto text-sm">
    {#each controller.diff as item (item.name)}
      <li><strong>{item.label}:</strong> {formatRefloatValue(item.before)} → {formatRefloatValue(item.after)}</li>
    {/each}
  </ul>
</ConfirmDialog>
