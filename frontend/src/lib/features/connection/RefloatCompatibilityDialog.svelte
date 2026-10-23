<script lang="ts">
  import type { RefloatCompatibility } from '$lib/backend/contract';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer, type Activation } from '$lib/ui/activation';

  let {
    open,
    boardName,
    compatibility,
    busy,
    error,
    onretry,
    onexit,
  }: {
    open: boolean;
    boardName: string;
    compatibility: RefloatCompatibility;
    busy: boolean;
    error: string | null;
    onretry: Activation;
    onexit: Activation;
  } = $props();
  let versionsOpen = $state(false);
  $effect(() => {
    if (!open) versionsOpen = false;
  });
  function toggleVersions() {
    versionsOpen = !versionsOpen;
  }
</script>

<ConfirmDialog
  {open}
  titleId="refloat-compatibility-title"
  title="Refloat version unsupported"
  description="{boardName} is running Refloat {compatibility.reportedVersion}, which this version of Float View doesn’t support."
  confirmLabel="Back to boards"
  cancelLabel="Check again"
  dismissLabel="Back to boards"
  appearance="warning"
  {busy}
  {error}
  onconfirm={onexit}
  oncancel={onretry}
  ondismiss={onexit}
>
  <p class="text-sm leading-6 text-muted">
    To connect, use a supported Refloat release or a Float View version that supports this package.
  </p>
  <button
    type="button"
    class="mt-3 touch-manipulation text-sm text-accent underline"
    aria-expanded={versionsOpen}
    aria-controls="supported-refloat-versions"
    onpointerdown={(event) => activateFromPointer(event, toggleVersions)}
    onclick={(event) => activateFromKeyboardOrAssistiveClick(event, toggleVersions)}>View supported versions</button
  >
  {#if versionsOpen}
    <div id="supported-refloat-versions" class="mt-3 text-sm leading-6 text-muted">
      <p class="font-medium text-foreground">Supported by this Float View build:</p>
      <p>{compatibility.supportedVersions.join(', ')}</p>
      <p class="mt-2">Beta and other prerelease packages are not supported.</p>
    </div>
  {/if}
</ConfirmDialog>
