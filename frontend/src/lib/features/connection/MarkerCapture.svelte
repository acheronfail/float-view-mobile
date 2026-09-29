<script lang="ts">
  import { onDestroy } from 'svelte';
  import { backendState } from '$lib/backend/state.svelte';
  import { ApiRequestError } from '$lib/backend/client';
  import { MarkerGesture } from './marker-gesture';
  let { sessionId }: { sessionId: string | null } = $props();
  const gesture = new MarkerGesture();
  let pending = $state(false);
  let pulse = $state(false);
  let message = $state('');
  let alive = true;
  let generation = 0;
  let pulseTimer: ReturnType<typeof setTimeout> | undefined;
  let toastTimer: ReturnType<typeof setTimeout> | undefined;
  function clear() {
    gesture.cancel();
    clearTimeout(pulseTimer);
    clearTimeout(toastTimer);
    pulse = false;
    message = '';
    generation++;
  }
  $effect(() => {
    void sessionId;
    clear();
  });
  onDestroy(() => {
    alive = false;
    clear();
  });
  async function save() {
    if (!sessionId || pending) return;
    const session = sessionId;
    const ticket = generation;
    pending = true;
    clearTimeout(pulseTimer);
    pulse = false;
    message = '';
    try {
      await backendState.saveRideMarker(session, crypto.randomUUID());
      if (!alive || session !== sessionId || ticket !== generation) return;
      pulse = true;
      clearTimeout(pulseTimer);
      pulseTimer = setTimeout(() => (pulse = false), 250);
      message = 'Marker saved';
    } catch (error) {
      if (!alive || session !== sessionId || ticket !== generation) return;
      message = error instanceof ApiRequestError ? 'Marker could not be saved' : 'Marker save not confirmed';
    } finally {
      pending = false;
      if (alive && ticket === generation) {
        clearTimeout(toastTimer);
        toastTimer = setTimeout(() => (message = ''), 2_000);
      }
    }
  }
</script>

<svelte:window onpointerdown={(event) => gesture.observePointer(event)} />

{#if pulse}<div class="marker-pulse pointer-events-none absolute inset-0" aria-hidden="true"></div>{/if}
<button
  type="button"
  aria-label="Save ride marker"
  disabled={!sessionId}
  aria-disabled={!sessionId || pending}
  class="absolute top-[53%] left-1/2 h-24 w-24 -translate-x-1/2 -translate-y-1/2 touch-manipulation rounded-full focus-visible:outline-2 focus-visible:outline-warning"
  onpointerdown={(event) => gesture.start(event)}
  onpointermove={(event) => gesture.move(event)}
  onpointerup={(event) => {
    if (gesture.end(event)) void save();
  }}
  onpointercancel={() => gesture.cancel()}
  onpointerleave={(event) => gesture.leave(event)}
  onkeydown={(event) => {
    if (event.repeat && (event.key === 'Enter' || event.key === ' ')) event.preventDefault();
  }}
  onclick={(event) => {
    if (event.detail === 0) void save();
  }}
></button>
{#if sessionId}<span
    class="pointer-events-none absolute bottom-0 left-1/2 -translate-x-1/2 text-[10px] whitespace-nowrap text-muted"
    >Double-tap to mark</span
  >{/if}
<div role="status" aria-live="polite" class="pointer-events-none absolute top-2 left-1/2 z-10 -translate-x-1/2">
  {#if message}<span
      class="marker-toast inline-block rounded-xl border px-3 py-2 text-sm font-semibold whitespace-nowrap"
      >{message}</span
    >{/if}
</div>

<style>
  .marker-pulse {
    background: radial-gradient(
      ellipse closest-side at 50% 53%,
      rgb(255 237 186 / 0.8) 0%,
      rgb(253 186 116 / 0.6) 18%,
      rgb(249 115 22 / 0.35) 42%,
      rgb(249 115 22 / 0.12) 65%,
      transparent 100%
    );
    filter: blur(10px);
    transform-origin: 50% 53%;
    animation: marker-pulse 250ms cubic-bezier(0.16, 1, 0.3, 1) both;
  }
  .marker-toast {
    color: #fff7ed;
    background: linear-gradient(135deg, #7c2d12, #431407);
    border-color: #fb923c;
    box-shadow:
      0 4px 16px rgb(0 0 0 / 0.3),
      0 0 18px rgb(249 115 22 / 0.3);
  }
  @keyframes marker-pulse {
    from {
      opacity: 1;
      transform: scale(0.35);
    }
    to {
      opacity: 0;
      transform: scale(1.15);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .marker-pulse {
      animation: none;
      opacity: 0.65;
    }
  }
</style>
