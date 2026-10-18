<script lang="ts">
  import { backDismissal } from '$lib/ui/back-dismissal';
  import { createMotion } from './motion';
  import { tick } from 'svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer, type Activation } from './activation';
  import { restoreDialogFocus, wrappedDialogFocus } from './dialog';

  const { appear: appearSurface, exit: exitSurface, surface: enterSurface } = createMotion();

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
  <div
    in:appearSurface={'content'}
    out:exitSurface
    class="fixed inset-0 z-50 flex items-center justify-center pt-[max(1rem,env(safe-area-inset-top))] pr-[max(1rem,env(safe-area-inset-right))] pb-[max(1rem,env(safe-area-inset-bottom))] pl-[max(1rem,env(safe-area-inset-left))]"
  >
    <button
      type="button"
      class="absolute inset-0 h-full w-full cursor-default touch-none bg-black/60"
      aria-label="Close help"
      onpointerdown={(event) => activateFromPointer(event, close)}
      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, close)}
    ></button>
    <div
      use:enterSurface={'dialog'}
      bind:this={dialog}
      use:backDismissal={{ close }}
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      aria-describedby={`${titleId}-description`}
      class="relative flex max-h-full min-h-0 w-full max-w-sm flex-col overflow-hidden rounded-2xl border border-outline bg-surface p-5 shadow-2xl"
    >
      <h2 id={titleId} class="shrink-0 text-xl font-semibold">{title}</h2>
      <!-- The scroll region needs focus so keyboard users can read the entire description. -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div
        id={`${titleId}-description`}
        role="region"
        aria-label={`${title} help`}
        tabindex="0"
        class="mt-2 min-h-0 touch-pan-y overflow-y-auto overscroll-contain text-sm leading-6 wrap-anywhere whitespace-pre-wrap text-muted"
      >
        {description}
      </div>
      <div class="mt-6 flex shrink-0 justify-end">
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
