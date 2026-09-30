<script lang="ts">
  import { tick, type Snippet } from 'svelte';
  import {
    activateFromKeyboardOrAssistiveClick,
    activateFromPointer,
    isPrimaryPointerActivation,
    type Activation,
  } from './activation';
  import { shouldDismissSheet } from './sheet';

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

  let sheet = $state<HTMLElement>();
  let closeButton = $state<HTMLButtonElement>();
  let dragOffset = $state(0);
  let dragging = $state(false);
  let closing = $state(false);
  let pointerId: number | undefined;
  let startY = 0;
  let lastY = 0;
  let lastTime = 0;
  let velocity = 0;
  let sheetHeight = 0;

  $effect(() => {
    void tick().then(() => closeButton?.focus());
  });

  async function close() {
    if (closing) return;
    closing = true;
    try {
      await onclose();
    } finally {
      dragOffset = 0;
      closing = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      void close();
      return;
    }
    if (event.key !== 'Tab' || !sheet) return;
    const controls = Array.from(
      sheet.querySelectorAll<HTMLElement>(
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

  function startDrag(event: PointerEvent) {
    if (!isPrimaryPointerActivation(event) || closing) return;
    event.preventDefault();
    const handle = event.currentTarget as HTMLElement;
    handle.setPointerCapture(event.pointerId);
    pointerId = event.pointerId;
    startY = event.clientY;
    lastY = event.clientY;
    lastTime = event.timeStamp;
    velocity = 0;
    sheetHeight = sheet?.offsetHeight ?? 0;
    dragOffset = 0;
    dragging = true;
  }

  function moveDrag(event: PointerEvent) {
    if (!dragging || event.pointerId !== pointerId) return;
    event.preventDefault();
    const elapsed = event.timeStamp - lastTime;
    if (elapsed > 0) velocity = (event.clientY - lastY) / elapsed;
    lastY = event.clientY;
    lastTime = event.timeStamp;
    dragOffset = Math.max(0, event.clientY - startY);
  }

  function finishDrag(event: PointerEvent) {
    if (!dragging || event.pointerId !== pointerId) return;
    const handle = event.currentTarget as HTMLElement;
    if (handle.hasPointerCapture(event.pointerId)) handle.releasePointerCapture(event.pointerId);
    pointerId = undefined;
    dragging = false;
    if (
      shouldDismissSheet({
        distancePx: dragOffset,
        heightPx: sheetHeight,
        velocityPxPerMs: velocity,
      })
    ) {
      dragOffset = sheetHeight;
      void close();
    } else {
      dragOffset = 0;
    }
  }

  function cancelDrag(event: PointerEvent) {
    if (!dragging || event.pointerId !== pointerId) return;
    pointerId = undefined;
    dragging = false;
    dragOffset = 0;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="fixed inset-0 z-40 h-screen bg-black/55">
  <button
    type="button"
    class="absolute inset-0 h-full w-full cursor-default touch-none"
    aria-label={`Close ${title}`}
    onpointerdown={(event) => activateFromPointer(event, close)}
    onclick={(event) => activateFromKeyboardOrAssistiveClick(event, close)}
  ></button>
  <div
    bind:this={sheet}
    role="dialog"
    aria-modal="true"
    aria-labelledby={titleId}
    class="absolute inset-x-0 bottom-0 flex max-h-[92%] flex-col overflow-hidden rounded-t-3xl border border-b-0 border-outline bg-surface shadow-2xl transition-transform duration-200 ease-out motion-reduce:transition-none"
    style:transform={`translate3d(0, ${dragOffset}px, 0)`}
    style:transition={dragging ? 'none' : undefined}
  >
    <div
      class="shrink-0 cursor-grab touch-none px-5 pt-3 pb-2 active:cursor-grabbing"
      onpointerdown={startDrag}
      onpointermove={moveDrag}
      onpointerup={finishDrag}
      onpointercancel={cancelDrag}
    >
      <div class="mx-auto h-1.5 w-12 rounded-full bg-outline"></div>
    </div>
    <div class="flex shrink-0 items-center justify-between px-5 pb-4">
      <h2 id={titleId} class="text-xl font-semibold">{title}</h2>
      <button
        bind:this={closeButton}
        type="button"
        class="touch-manipulation rounded-full border border-outline px-3 py-1.5"
        disabled={closing}
        onpointerdown={(event) => activateFromPointer(event, close)}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, close)}
        >{closing ? 'Saving…' : closeLabel}</button
      >
    </div>
    <div class="overflow-y-auto px-5 pb-[max(1.25rem,env(safe-area-inset-bottom))]">
      {@render children()}
    </div>
  </div>
</div>
