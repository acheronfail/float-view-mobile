<script lang="ts">
  import { tick } from 'svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer, type Activation } from '$lib/ui/activation';

  let {
    busy = false,
    disabled = false,
    ondisable,
  }: {
    busy?: boolean;
    disabled?: boolean;
    ondisable: Activation;
  } = $props();

  let disableButton = $state<HTMLButtonElement>();

  $effect(() => {
    if (busy || disabled || typeof document === 'undefined') return;
    void tick().then(() => disableButton?.focus());
  });

  function disable() {
    if (!busy && !disabled) void ondisable();
  }

  function containFocus(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      return;
    }
    if (event.key !== 'Tab') return;
    event.preventDefault();
    disableButton?.focus();
  }
</script>

<svelte:window onkeydown={containFocus} />

<div class="fixed inset-0 z-50 grid h-screen place-items-center bg-black/75 p-4">
  <div
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="flywheel-mode-title"
    aria-describedby="flywheel-mode-description"
    class="w-full max-w-sm rounded-2xl border border-warning bg-surface p-6 text-center shadow-2xl"
  >
    <h2 id="flywheel-mode-title" class="text-2xl font-semibold">Flywheel mode is active</h2>
    <p id="flywheel-mode-description" class="mt-3 text-sm leading-6 text-muted">
      Disengage the board before disabling flywheel mode.
    </p>
    <button
      bind:this={disableButton}
      type="button"
      class="text-on-warning mt-6 w-full touch-manipulation rounded-lg bg-warning px-4 py-3 font-semibold"
      disabled={busy || disabled}
      onpointerdown={(event) => activateFromPointer(event, disable)}
      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, disable)}>Disable</button
    >
  </div>
</div>
