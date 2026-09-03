<script lang="ts">
  import { tick } from 'svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer, type Activation } from './activation';
  import { restoreDialogFocus, wrappedDialogFocus } from './dialog';

  let {
    open,
    titleId,
    title,
    description,
    returnFocus,
    onclose,
  }: {
    open: boolean;
    titleId: string;
    title: string;
    description: string;
    returnFocus?: HTMLElement;
    onclose: Activation;
  } = $props();

  let dialog = $state<HTMLElement>();
  let doneButton = $state<HTMLButtonElement>();

  $effect(() => {
    if (!open || typeof document === 'undefined') return;
    const focusTarget = returnFocus;
    void tick().then(() => doneButton?.focus());
    return () => restoreDialogFocus(focusTarget);
  });

  function close() {
    void onclose();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!open) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      close();
      return;
    }
    if (event.key !== 'Tab' || !dialog) return;
    const controls = Array.from(
      dialog.querySelectorAll<HTMLElement>(
        'button:not(:disabled), [href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])'
      )
    );
    const activeElement = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const target = wrappedDialogFocus(activeElement, controls, event.shiftKey);
    if (!target) return;
    event.preventDefault();
    target.focus();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class="fixed inset-0 z-50 grid place-items-center p-4">
    <button
      type="button"
      class="absolute inset-0 h-full w-full cursor-default touch-none bg-black/60"
      aria-label="Close help"
      onpointerdown={(event) => activateFromPointer(event, close)}
      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, close)}
    ></button>
    <div
      bind:this={dialog}
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      aria-describedby={`${titleId}-description`}
      class="relative w-full max-w-sm rounded-2xl border border-outline bg-surface p-5 shadow-2xl"
    >
      <h2 id={titleId} class="text-xl font-semibold">{title}</h2>
      <p id={`${titleId}-description`} class="mt-2 text-sm leading-6 whitespace-pre-wrap text-muted">{description}</p>
      <div class="mt-6 flex justify-end">
        <button
          bind:this={doneButton}
          type="button"
          class="touch-manipulation rounded-lg bg-accent px-4 py-2 font-medium text-on-accent"
          onpointerdown={(event) => activateFromPointer(event, close)}
          onclick={(event) => activateFromKeyboardOrAssistiveClick(event, close)}>Done</button
        >
      </div>
    </div>
  </div>
{/if}
