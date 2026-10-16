<script lang="ts">
  import { backDismissal } from '$lib/ui/back-dismissal';
  import { onMount } from 'svelte';
  import { backendState } from '$lib/backend/state.svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import Icon from '$lib/ui/Icon.svelte';
  import { createMotion } from '$lib/ui/motion';
  import { wrappedDialogFocus } from '$lib/ui/dialog';
  import { rideLocationPresentation } from './location-presentation';

  let {
    open = $bindable(false),
    unavailable = false,
    onclose,
  }: {
    open?: boolean;
    unavailable?: boolean;
    onclose: () => void;
  } = $props();
  const motion = createMotion();
  const titleId = $props.id();
  const presentation = $derived(rideLocationPresentation(backendState.snapshot?.location));
  let dialog: HTMLDialogElement;
  let button = $state<HTMLButtonElement>();
  let fromHeader = false;
  let pending = $state(false);
  let error = $state<string | null>(null);

  onMount(() => () => dialog.close());
  $effect(() => {
    if (unavailable) open = false;
    if (open && !dialog.open) {
      error = null;
      dialog.showModal();
    }
    if (!open && dialog.open) dialog.close();
  });

  function show() {
    if (unavailable) return;
    fromHeader = true;
    open = true;
  }

  function close(keyboard: boolean) {
    dialog.close();
    open = false;
    if (keyboard) {
      if (fromHeader && button?.isConnected) button.focus();
      else onclose();
    } else button?.blur();
    fromHeader = false;
  }

  async function activate() {
    if (pending || backendState.busy || presentation.action === 'none') return;
    pending = true;
    error = null;
    try {
      if (presentation.action === 'enable') {
        await backendState.updateSessionSettings({ locationRecordingEnabled: true });
      } else {
        await backendState.requestLocationPermission();
      }
      error = backendState.error;
    } finally {
      pending = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key !== 'Tab') return;
    const controls = Array.from(dialog.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'));
    const active = document.activeElement instanceof HTMLButtonElement ? document.activeElement : null;
    const target = wrappedDialogFocus(active, controls, event.shiftKey);
    if (target) {
      event.preventDefault();
      target.focus();
    }
  }
</script>

{#if !presentation.enabled}
  <button
    bind:this={button}
    type="button"
    class="relative grid h-11 w-11 shrink-0 touch-manipulation place-items-center rounded-full border border-warning/40 bg-warning/10 text-warning disabled:opacity-50"
    aria-label={presentation.label}
    title={presentation.label}
    aria-haspopup="dialog"
    aria-expanded={open}
    disabled={unavailable}
    onpointerdown={(event) => activateFromPointer(event, show)}
    onclick={(event) => activateFromKeyboardOrAssistiveClick(event, show)}
  >
    <Icon name={presentation.off ? 'map-pin-off' : 'map-pin'} class="text-xl" />
    {#if presentation.warning}<span class="absolute top-1 right-1 h-2 w-2 rounded-full bg-warning" aria-hidden="true"
      ></span>{/if}
  </button>
{/if}

<dialog
  use:backDismissal={{ close: () => close(true), enabled: open }}
  bind:this={dialog}
  use:motion.surface={'dialog'}
  aria-labelledby={titleId}
  aria-describedby={`${titleId}-description`}
  class="fixed inset-0 m-auto max-h-[calc(100dvh-2rem)] w-[calc(100%-2rem)] max-w-sm overflow-y-auto rounded-2xl border border-outline bg-surface p-5 text-foreground shadow-2xl backdrop:bg-black/60"
  onkeydown={handleKeydown}
  oncancel={(event) => {
    event.preventDefault();
    event.stopPropagation();
    close(true);
  }}
>
  <h2 id={titleId} class="text-xl font-semibold">Ride location</h2>
  <p
    class="mt-2 text-sm font-medium"
    class:text-warning={presentation.warning}
    class:text-muted={presentation.off}
    class:text-success={presentation.enabled}
  >
    {presentation.off ? 'Recording off' : presentation.enabled ? 'Recording enabled' : 'Recording unavailable'}
  </p>
  <p id={`${titleId}-description`} class="mt-2 text-sm leading-6 text-muted">{presentation.description}</p>
  {#if error}<p role="alert" class="mt-3 text-sm text-warning">{error}</p>{/if}
  <div class="mt-6 flex justify-end gap-3">
    <button
      type="button"
      class="min-h-11 touch-manipulation rounded-lg border border-outline px-4 py-2"
      onpointerdown={(event) => activateFromPointer(event, () => close(false))}
      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => close(true))}>Done</button
    >
    {#if presentation.action !== 'none'}
      <button
        type="button"
        class="min-h-11 touch-manipulation rounded-lg bg-accent px-4 py-2 font-medium text-on-accent disabled:opacity-50"
        disabled={pending || backendState.busy}
        onpointerdown={(event) => activateFromPointer(event, activate)}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, activate)}
        >{pending ? 'Working…' : presentation.actionLabel}</button
      >
    {/if}
  </div>
</dialog>
