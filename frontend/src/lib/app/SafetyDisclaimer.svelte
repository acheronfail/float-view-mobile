<script lang="ts">
  import { tick } from 'svelte';
  import { backDismissal } from '$lib/ui/back-dismissal';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import { wrappedDialogFocus } from '$lib/ui/dialog';

  let {
    busy,
    error,
    onaccept,
  }: {
    busy: boolean;
    error: string;
    onaccept: () => Promise<void>;
  } = $props();
  let dialog = $state<HTMLElement>();
  let acceptButton = $state<HTMLButtonElement>();
  $effect(() => {
    void tick().then(() => acceptButton?.focus());
  });

  function keydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      return;
    }
    if (event.key !== 'Tab' || !dialog) return;
    const controls = [...dialog.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')];
    const target = wrappedDialogFocus(
      document.activeElement instanceof HTMLButtonElement ? document.activeElement : null,
      controls,
      event.shiftKey
    );
    if (target) {
      event.preventDefault();
      target.focus();
    }
  }
</script>

<svelte:window onkeydown={keydown} />

<div class="fixed inset-0 z-50 grid place-items-center overflow-y-auto bg-black/60 p-4">
  <div
    bind:this={dialog}
    use:backDismissal={{ close: () => {} }}
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="safety-title"
    aria-describedby="safety-description"
    aria-busy={busy}
    class="my-auto w-full max-w-md rounded-2xl border border-outline bg-surface p-5 shadow-2xl"
  >
    <h2 id="safety-title" class="text-xl font-semibold">Ride safely</h2>
    <div id="safety-description" class="mt-3 space-y-3 text-sm leading-6 text-muted">
      <p>
        Riding can cause serious injury or death. Wear a helmet and protective gear, follow local laws, and ride within
        your limits. Stop in a safe place before using Float View.
      </p>
      <p>
        Float View cannot guarantee your safety or the accuracy of board data. You use this app and ride at your own
        risk. To the extent permitted by law, the app’s author accepts no responsibility for injury, death, or damage
        arising from its use.
      </p>
      <p>
        By choosing “I agree”, you acknowledge these risks and accept responsibility for your riding and use of the app.
      </p>
    </div>
    {#if error}<p role="alert" class="mt-3 text-sm text-danger">{error}</p>{/if}
    <div class="mt-5 flex flex-wrap justify-end gap-3">
      <button
        bind:this={acceptButton}
        type="button"
        class="touch-manipulation rounded-lg bg-accent px-4 py-2 font-medium text-on-accent"
        disabled={busy}
        onpointerdown={(event) => activateFromPointer(event, onaccept)}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, onaccept)}
        >{busy ? 'Working…' : 'I agree'}</button
      >
    </div>
  </div>
</div>
