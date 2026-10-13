<script lang="ts">
  import { createMotion } from './motion';
  import { tick } from 'svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer, type Activation } from './activation';

  const { appear: appearSurface, exit: exitSurface, surface: enterSurface } = createMotion();

  let {
    open,
    boardName,
    busy = false,
    oncancel,
    onguest,
    onsave,
  }: {
    open: boolean;
    boardName: string;
    busy?: boolean;
    oncancel: Activation;
    onguest: Activation;
    onsave: Activation;
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

  function guest() {
    if (!busy) void onguest();
  }

  function save() {
    if (!busy) void onsave();
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
      dialog.querySelectorAll<HTMLElement>('button:not(:disabled), [href], input:not(:disabled), select:not(:disabled)')
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
  <div in:appearSurface={'content'} out:exitSurface class="fixed inset-0 z-50 grid place-items-center p-4">
    <button
      type="button"
      class="absolute inset-0 h-full w-full cursor-default touch-none bg-black/60"
      aria-label="Cancel new board"
      onpointerdown={(event) => activateFromPointer(event, cancel)}
      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, cancel)}
    ></button>
    <div
      use:enterSurface={'dialog'}
      bind:this={dialog}
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="new-board-title"
      aria-describedby="new-board-description"
      class="relative w-full max-w-sm rounded-2xl border border-outline bg-surface p-5 shadow-2xl"
    >
      <h2 id="new-board-title" class="text-xl font-semibold">New board</h2>
      <p id="new-board-description" class="mt-2 text-sm leading-6 text-muted">
        {boardName} is not saved yet. Guest mode shows the live view, but nothing will be saved or tracked.
      </p>
      <div class="mt-6 flex flex-col gap-2">
        <button
          bind:this={cancelButton}
          type="button"
          class="touch-manipulation rounded-lg border border-outline px-4 py-2 text-left"
          disabled={busy}
          onpointerdown={(event) => activateFromPointer(event, cancel)}
          onclick={(event) => activateFromKeyboardOrAssistiveClick(event, cancel)}>Cancel</button
        >
        <button
          type="button"
          class="touch-manipulation rounded-lg border border-outline px-4 py-2 text-left"
          disabled={busy}
          onpointerdown={(event) => activateFromPointer(event, guest)}
          onclick={(event) => activateFromKeyboardOrAssistiveClick(event, guest)}>Connect as guest</button
        >
        <button
          type="button"
          class="touch-manipulation rounded-lg bg-accent px-4 py-2 text-left font-medium text-on-accent"
          disabled={busy}
          onpointerdown={(event) => activateFromPointer(event, save)}
          onclick={(event) => activateFromKeyboardOrAssistiveClick(event, save)}>Save and connect</button
        >
      </div>
      {#if busy}<p class="mt-3 text-sm text-muted" role="status">Connecting…</p>{/if}
    </div>
  </div>
{/if}
