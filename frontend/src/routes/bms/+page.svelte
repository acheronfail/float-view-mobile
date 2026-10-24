<script lang="ts">
  import { getContext, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { connectionReturnHref } from '$lib/app/connection-navigation';
  import { APP_HEADER_CONTEXT, type AppHeaderContext } from '$lib/app/header';
  import { backendState } from '$lib/backend/state.svelte';
  import BmsView from '$lib/features/connection/BmsView.svelte';

  const appHeader = getContext<AppHeaderContext>(APP_HEADER_CONTEXT);
  const headerOwner = '/bms';
  const connection = $derived(backendState.snapshot?.activeConnection);
  let disconnecting = $state(false);

  $effect(() => {
    if (!connection) return;
    appHeader.set(headerOwner, {
      title: connection.displayName,
      subtitle: connection.state === 'ready' ? 'Battery management system' : connection.state.replaceAll('_', ' '),
      subtitleClass: connection.state === 'ready' ? 'text-sm text-success' : 'text-sm capitalize text-warning',
      menuLabel: 'BMS actions',
      menu: [
        {
          label: 'Go to Sessions',
          icon: 'list',
          onactivate: (closeMenu: () => void) => {
            closeMenu();
            void goto(connectionReturnHref);
          },
        },
        {
          label: 'Disconnect',
          appearance: 'destructive',
          disabled: backendState.busy || disconnecting,
          onactivate: (closeMenu: () => void) => {
            closeMenu();
            void disconnect();
          },
        },
      ],
    });
  });
  onDestroy(() => appHeader.clear(headerOwner));

  async function disconnect() {
    if (disconnecting) return;
    disconnecting = true;
    await backendState.disconnectDevice();
    disconnecting = false;
    if (!backendState.snapshot?.activeConnection) await goto('#/', { replaceState: true });
  }
</script>

<section aria-labelledby="app-title" class="flex flex-1 flex-col gap-4 px-4 py-3 sm:px-6">
  {#if connection?.deviceKind === 'bms'}
    <BmsView bms={connection.bms} units={backendState.settings?.unitPreference ?? 'metric'} />
  {:else}
    <p class="text-muted">Identifying battery management system…</p>
  {/if}
  {#if backendState.error}<p role="alert" class="text-warning">{backendState.error}</p>{/if}
</section>
