<script lang="ts">
  import { onMount } from 'svelte';
  import { backendState } from '$lib/backend/state.svelte';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import RefloatConfigurationField from './RefloatConfigurationField.svelte';
  import { formatRefloatValue, type RefloatConfigurationController } from './refloat-configuration.svelte';

  let { controller }: { controller: RefloatConfigurationController } = $props();
  let writeConfirmation = $state(false);

  onMount(() => {
    void controller.open();
  });

  export function hasUnsavedChanges(): boolean {
    return controller.diff.length > 0;
  }

  async function write() {
    if (await controller.write()) writeConfirmation = false;
  }
</script>

<section class="flex min-h-[70vh] flex-col gap-3" aria-labelledby="refloat-configuration-title">
  <div class="sticky top-0 z-10 grid gap-2 bg-surface pb-2 sm:grid-cols-[1fr_auto_auto]">
    <label class="text-sm">
      <span class="sr-only">Search configuration fields</span>
      <input
        type="search"
        placeholder="Search fields"
        class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
        bind:value={controller.search}
      />
    </label>
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
    <div class="grid gap-4">
      {#each controller.filteredPages as page (page.group + page.subgroup)}
        <section class="grid gap-2" aria-labelledby={`refloat-${page.group}-${page.subgroup}`}>
          <h3 id={`refloat-${page.group}-${page.subgroup}`} class="text-lg font-semibold">
            {page.group} · {page.subgroup}
          </h3>
          <div class="grid gap-2 md:grid-cols-2">
            {#each page.fields as field (field.name)}
              <RefloatConfigurationField
                {field}
                value={controller.draft[field.name]}
                error={controller.validationErrors[field.name]}
                disabled={controller.loading || backendState.busy}
                onchange={(value) => controller.setValue(field.name, value)}
              />
            {/each}
          </div>
        </section>
      {/each}
    </div>
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
