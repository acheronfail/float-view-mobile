<script lang="ts">
  import { tick, type Snippet } from 'svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer, type Activation } from './activation';
  import { wrappedDialogFocus } from './dialog';

  let {
    title,
    titleId,
    closeLabel = 'Done',
    onclose,
    children,
  }: {
    title: string;
    titleId: string;
    closeLabel?: string;
    onclose: Activation;
    children: Snippet;
  } = $props();

  let dialog = $state<HTMLElement>();
  let closeButton = $state<HTMLButtonElement>();
  let closing = $state(false);

  $effect(() => {
    void tick().then(() => closeButton?.focus());
  });

  async function close() {
    if (closing) return;
    closing = true;
    try {
      await onclose();
    } finally {
      closing = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    // A confirmation above this overlay owns its own keyboard handling.
    if (event.defaultPrevented) return;
    if (document.activeElement?.closest('[role="dialog"], [role="alertdialog"]') !== dialog) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      void close();
      return;
    }
    if (event.key !== 'Tab' || !dialog) return;
    const controls = Array.from(
      dialog.querySelectorAll<HTMLElement>(
        'button:not(:disabled), [href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])'
      )
    );
    const target = wrappedDialogFocus(
      document.activeElement instanceof HTMLElement ? document.activeElement : null,
      controls,
      event.shiftKey
    );
    if (target) {
      event.preventDefault();
      target.focus();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  bind:this={dialog}
  role="dialog"
  aria-modal="true"
  aria-labelledby={titleId}
  class="fixed inset-0 z-40 h-dvh w-screen bg-canvas text-foreground"
>
  <div class="app-safe-area app-safe-area-without-tabs mx-auto flex h-full min-h-0 max-w-3xl flex-col">
    <header class="flex shrink-0 items-center justify-between gap-3 border-b border-outline pb-3">
      <h2 id={titleId} class="min-w-0 text-xl font-bold">{title}</h2>
      <button
        bind:this={closeButton}
        type="button"
        class="min-h-11 shrink-0 touch-manipulation rounded-lg border border-outline px-4"
        disabled={closing}
        onpointerdown={(event) => activateFromPointer(event, close)}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, close)}
        >{closing ? 'Saving…' : closeLabel}</button
      >
    </header>
    <div class="min-h-0 flex-1 overflow-y-auto overscroll-contain py-3">
      {@render children()}
    </div>
  </div>
</div>
