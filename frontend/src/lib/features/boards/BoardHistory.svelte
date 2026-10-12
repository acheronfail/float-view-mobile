<script lang="ts">
  import { getContext, onDestroy, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { APP_HEADER_CONTEXT, type AppHeaderContext } from '$lib/app/header';
  import { backendState } from '$lib/backend/state.svelte';
  import { formatDistanceM } from '$lib/features/measurements/format';
  import SessionList from '$lib/features/sessions/SessionList.svelte';
  import FullScreenOverlay from '$lib/ui/FullScreenOverlay.svelte';
  import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
  import BoardEditor from './BoardEditor.svelte';

  let { savedDeviceId }: { savedDeviceId: string } = $props();
  let configureOpen = $state(false);
  let forgetConfirmationOpen = $state(false);
  let forgetting = $state(false);
  let editor = $state<BoardEditor>();
  const appHeader = getContext<AppHeaderContext>(APP_HEADER_CONTEXT);
  const headerOwner = '/sessions/boards/[savedDeviceId]';
  const board = $derived(backendState.savedDevices.find((value) => value.savedDeviceId === savedDeviceId));
  const active = $derived(backendState.snapshot?.activeConnection?.savedDeviceId === savedDeviceId);
  const units = $derived(backendState.settings?.unitPreference ?? 'metric');
  const dateFormatter = new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' });

  function duration(milliseconds: number): string {
    const minutes = Math.round(milliseconds / 60_000);
    if (minutes < 60) return `${minutes} min`;
    return `${Math.floor(minutes / 60)} h ${minutes % 60} min`;
  }

  function formatDate(value: string | null): string {
    if (!value) return 'No rides yet';
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? 'Unknown' : dateFormatter.format(date);
  }

  $effect(() => {
    if (!board) {
      appHeader.set(headerOwner, { title: 'Board', backHref: '#/sessions', backLabel: 'Sessions' });
      return;
    }
    appHeader.set(headerOwner, {
      title: board.displayName,
      backHref: '#/sessions',
      backLabel: 'Sessions',
      menuLabel: 'Board actions',
      menu: [
        {
          label: 'Configure board',
          appearance: board.profileComplete ? ('normal' as const) : ('warning' as const),
          disabled: backendState.busy,
          onactivate: (closeMenu: () => void) => openConfigure(closeMenu),
        },
        {
          label: 'Forget board',
          appearance: 'destructive' as const,
          disabled: active || backendState.busy,
          onactivate: (closeMenu: () => void) => requestForget(closeMenu),
        },
      ],
    });
  });

  onDestroy(() => appHeader.clear(headerOwner));

  $effect(() => {
    if (backendState.status === 'connected' && !board) {
      void goto('#/sessions', { replaceState: true });
    }
  });

  function openConfigure(closeMenu: () => void) {
    closeMenu();
    configureOpen = true;
  }

  function requestForget(closeMenu: () => void) {
    closeMenu();
    if (!active) forgetConfirmationOpen = true;
  }

  async function closeConfigure() {
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    await tick();
    await editor?.flush();
    configureOpen = false;
    await tick();
    appHeader.focusMenu();
  }

  async function forgetBoard() {
    if (forgetting || active) return;
    forgetting = true;
    const deleted = await backendState.deleteSavedDevice(savedDeviceId);
    forgetting = false;
    forgetConfirmationOpen = false;
    if (deleted) await goto('#/sessions', { replaceState: true });
  }

  async function cancelForget() {
    forgetConfirmationOpen = false;
    await tick();
    appHeader.focusMenu();
  }
</script>

{#if board}
  <section aria-labelledby="app-title" class="space-y-5 py-2">
    <section aria-label="Board summary" class="grid grid-cols-2 gap-3">
      <div class="rounded-lg border border-outline bg-elevated p-3">
        <p class="text-sm text-muted">Sessions</p>
        <p class="text-xl">{board.summary.sessionCount}</p>
      </div>
      <div class="rounded-lg border border-outline bg-elevated p-3">
        <p class="text-sm text-muted">Riding time</p>
        <p class="text-xl">{duration(board.summary.totalRidingElapsedMs)}</p>
      </div>
      <div class="rounded-lg border border-outline bg-elevated p-3">
        <p class="text-sm text-muted">Distance</p>
        <p class="text-xl">{formatDistanceM(board.summary.totalDistanceM, units)}</p>
      </div>
      <div class="rounded-lg border border-outline bg-elevated p-3">
        <p class="text-sm text-muted">Most recent</p>
        <p class="text-xl">{formatDate(board.summary.mostRecentSessionAt)}</p>
      </div>
    </section>

    <section aria-label="Board sessions" class="space-y-3">
      <h2 class="text-xl font-semibold">Sessions</h2>
      <SessionList {savedDeviceId} />
    </section>
  </section>
{:else if backendState.status === 'connected'}
  <section class="rounded-2xl border border-outline bg-surface p-5">
    <p role="alert" class="text-warning">This saved board is no longer available.</p>
  </section>
{:else}
  <p class="text-muted">Loading saved board…</p>
{/if}

<ConfirmDialog
  open={forgetConfirmationOpen}
  titleId="forget-board-confirmation-title"
  title="Forget board?"
  description="This will also remove all saved sessions from this board. This cannot be undone."
  confirmLabel="Forget board"
  appearance="destructive"
  busy={forgetting}
  onconfirm={forgetBoard}
  oncancel={cancelForget}
/>

{#if configureOpen && board}
  <FullScreenOverlay title="Board configuration" titleId="board-history-configure-title" onclose={closeConfigure}>
    <BoardEditor bind:this={editor} {savedDeviceId} embedded onclose={closeConfigure} />
  </FullScreenOverlay>
{/if}
