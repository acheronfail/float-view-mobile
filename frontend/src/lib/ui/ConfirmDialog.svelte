
<script lang="ts" module>
  export type DialogAppearance = 'normal' | 'warning' | 'destructive';
</script>
<script lang="ts">
  import { tick } from 'svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer, type Activation } from './activation';

  let {
    open,
    titleId,
    title,
    description,
    confirmLabel = 'Confirm',
    cancelLabel = 'Cancel',
    appearance = 'normal',
    busy = false,
    onconfirm,
    oncancel,
  }: {
    open: boolean;
    titleId: string;
    title: string;
    description: string;
    confirmLabel?: string;
    cancelLabel?: string;
    appearance?: DialogAppearance;
    busy?: boolean;
    onconfirm: Activation;
    oncancel: Activation;
  } = $props();

  let dialog = $state<HTMLElement>();
  let cancelButton = $state<HTMLButtonElement>();

  $effect(() => {
    if (!open || typeof document === 'undefined') return;
    const previouslyFocused = document.activeElement instanceof HTMLElement ? document.activeElement : undefined;
    void tick().then(() => cancelButton?.focus());
    return () => previouslyFocused?.focus();
  });

  function cancel() {
    if (!busy) void oncancel();
  }

  function confirm() {
    if (!busy) void onconfirm();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!open) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      cancel();
      return;
    }
    if (event.key !== 'Tab' || !dialog) return;
    const controls = Array.from(
      dialog.querySelectorAll<HTMLElement>(
        'button:not(:disabled), [href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])'
      )
    );
    const first = controls[0];
    const last = controls.at(-1);
    if (!first || !last) return;
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class="fixed inset-0 z-50 grid place-items-center p-4">
    <button
      type="button"
      class="absolute inset-0 h-full w-full cursor-default touch-none bg-black/60"
      aria-label={cancelLabel}
      onpointerdown={(event) => activateFromPointer(event, cancel)}
      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, cancel)}
    ></button>
    <div
      bind:this={dialog}
      role="alertdialog"
      aria-modal="true"
      aria-labelledby={titleId}
      aria-describedby={`${titleId}-description`}
      class="relative w-full max-w-sm rounded-2xl border border-outline bg-surface p-5 shadow-2xl"
    >
      <h2 id={titleId} class="text-xl font-semibold">{title}</h2>
      <p id={`${titleId}-description`} class="mt-2 text-sm leading-6 text-muted">{description}</p>
      <div class="mt-6 flex justify-end gap-3">
        <button
          bind:this={cancelButton}
          type="button"
          class="touch-manipulation rounded-lg border border-outline px-4 py-2"
          disabled={busy}
          onpointerdown={(event) => activateFromPointer(event, cancel)}
          onclick={(event) => activateFromKeyboardOrAssistiveClick(event, cancel)}>{cancelLabel}</button
        >
        <button
          type="button"
          class="touch-manipulation rounded-lg px-4 py-2 font-medium"
          class:bg-danger={appearance === 'destructive'}
          class:text-on-danger={appearance === 'destructive'}
          class:bg-warning={appearance === 'warning'}
          class:text-on-warning={appearance === 'warning'}
          class:bg-accent={appearance === 'normal'}
          class:text-on-accent={appearance === 'normal'}
          disabled={busy}
          onpointerdown={(event) => activateFromPointer(event, confirm)}
          onclick={(event) => activateFromKeyboardOrAssistiveClick(event, confirm)}
          >{busy ? 'Working…' : confirmLabel}</button
        >
      </div>
    </div>
  </div>
{/if}
