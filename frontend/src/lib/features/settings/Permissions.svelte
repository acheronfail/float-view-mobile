<script lang="ts">
  import { backendState } from '$lib/backend/state.svelte';
  import type { PermissionGuidance, PermissionRow } from '$lib/backend/contract';
  import Icon from '$lib/ui/Icon.svelte';
  import type { IconName } from '$lib/ui/icon';
  import SectionTitle from '$lib/ui/SectionTitle.svelte';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';

  const names = {
    bluetooth: 'Bluetooth',
    location: 'Location',
    backgroundLocation: 'Background location',
    notifications: 'Notifications',
    liveUpdates: 'Live Updates',
  };
  const grants = {
    unknown: 'Unknown',
    notDetermined: 'Not requested',
    granted: 'Allowed',
    denied: 'Not allowed',
    restricted: 'Restricted',
    notRequired: 'Not required',
  };
  const purposes = {
    bluetooth: 'Discover and connect to your board.',
    location: 'Record the route of your rides.',
    backgroundLocation: 'Record routes in the background or with your phone locked.',
    notifications: 'See ride readings in notifications and on the lock screen.',
    liveUpdates: 'Show prominent ride updates on supported devices.',
  };
  const statuses = {
    notDetermined: 'Not requested',
    granted: 'Allowed',
    limited: 'Limited access',
    denied: 'Not allowed',
    restricted: 'Restricted',
    unavailable: 'Unavailable',
  };
  const icons: Record<PermissionRow['permission'], IconName> = {
    bluetooth: 'bluetooth',
    location: 'map-pin',
    backgroundLocation: 'route',
    notifications: 'bell',
    liveUpdates: 'signal',
  };
  const statusStyles: Record<PermissionRow['status'], string> = {
    notDetermined: 'bg-elevated text-muted',
    granted: 'bg-success/10 text-success',
    limited: 'bg-warning/10 text-warning',
    denied: 'bg-elevated text-muted',
    restricted: 'bg-warning/10 text-warning',
    unavailable: 'bg-elevated text-muted',
  };
  const statusIcons: Record<PermissionRow['status'], IconName> = {
    notDetermined: 'ellipsis-vertical',
    granted: 'check',
    limited: 'triangle-alert',
    denied: 'lock-keyhole',
    restricted: 'lock-keyhole',
    unavailable: 'triangle-alert',
  };
  const guidance: Record<PermissionGuidance, string> = {
    none: '',
    checking: 'Couldn’t check permission. Return to the app to try again.',
    restricted: 'Your device restricts this permission.',
    unsupportedHardware: 'Bluetooth isn’t available on this device.',
    servicesDisabled: 'Turn on Location Services in your device’s settings.',
    foregroundRequired: 'Allow Location first to enable background recording.',
    rationale: 'Your device will explain why this permission is needed.',
    iosSettings: 'In Settings, open Float View and select Location or enable Bluetooth.',
    appSettings: 'Open the app’s system settings, then Permissions, and enable this access.',
    macBluetooth: 'In System Settings, open Privacy & Security → Bluetooth and enable Float View.',
    macLocation: 'In System Settings, open Privacy & Security → Location Services and enable Float View.',
    backgroundSettings: 'In app settings, open Permissions → Location and allow background access.',
    alwaysAccess: 'Choose Always to record rides that start with your phone locked.',
    notificationSettings: 'Manage Float View’s ride notifications in system settings.',
    liveUpdateSettings: 'Allow Live Updates in system settings.',
    notificationsRequired: 'Allow Notifications first to enable Live Updates.',
    fineLocationForScan: 'This Android version also needs precise location to scan for Bluetooth boards.',
  };
  function guidanceText(row: PermissionRow) {
    if (row.guidance === 'servicesDisabled' && row.permission === 'bluetooth') {
      return 'Turn on Bluetooth in your device’s settings. Bluetooth permission is already allowed.';
    }
    if (row.guidance === 'iosSettings') {
      return row.permission === 'bluetooth'
        ? 'In Settings, open Float View and allow Bluetooth.'
        : 'In Settings, open Float View → Location.';
    }
    return guidance[row.guidance];
  }
  function recover(row: PermissionRow) {
    return backendState.recoverPermission(row);
  }
</script>

<section aria-label="Permissions">
  <SectionTitle title="Permissions" />
  {#if backendState.permissionError && backendState.status === 'connected'}
    <p role="alert" class="mt-3 rounded-xl border border-danger/30 bg-danger/10 p-3 text-sm text-danger">
      {backendState.permissionError}
    </p>
  {/if}
  <div class="mt-3 grid gap-3 sm:grid-cols-2">
    {#each backendState.snapshot?.permissions ?? [] as row (row.permission)}
      <article
        class="flex min-w-0 flex-col rounded-xl border border-outline bg-surface"
        data-permission={row.permission}
        aria-labelledby={`permission-${row.permission}`}
        aria-busy={row.pending}
      >
        <div class="flex-1 space-y-3 p-4">
          <div class="flex items-start gap-3">
            <div
              class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-accent-soft text-on-accent-soft"
            >
              <Icon name={icons[row.permission]} class="size-5" />
            </div>
            <div class="min-w-0 flex-1">
              <h3 id={`permission-${row.permission}`} class="text-sm font-semibold">{names[row.permission]}</h3>
              <div class="mt-1.5 flex flex-wrap items-center gap-2">
                <span
                  class={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium ${
                    row.pending ? 'bg-accent-soft text-on-accent-soft' : statusStyles[row.status]
                  }`}
                  role="status"
                  data-permission-status
                >
                  <Icon
                    name={row.pending ? 'loader-circle' : statusIcons[row.status]}
                    class={row.pending ? 'size-3.5 motion-safe:animate-spin' : 'size-3.5'}
                  />
                  {row.pending ? 'Waiting…' : statuses[row.status]}
                </span>
                {#if row.optional}<span class="text-xs text-muted">Optional</span>{/if}
              </div>
            </div>
          </div>
          <p class="text-sm leading-5 text-muted">{purposes[row.permission]}</p>
          {#if row.accuracy !== 'unknown' || row.scope !== 'unknown' || (row.scan && (row.scan !== 'granted' || row.connect !== 'granted')) || (row.connect && !row.scan)}
            <div class="flex flex-wrap gap-x-3 gap-y-1 text-xs text-muted">
              {#if row.accuracy !== 'unknown'}<span
                  >{row.accuracy === 'precise' ? 'Precise' : 'Approximate'} location</span
                >{/if}
              {#if row.scope !== 'unknown'}<span>{row.scope === 'always' ? 'Always' : 'While using the app'}</span>{/if}
              {#if row.scan && (row.scan !== 'granted' || row.connect !== 'granted')}
                <span>Scan: {grants[row.scan]}</span>
                <span>Connect: {row.connect ? grants[row.connect] : 'Unknown'}</span>
              {:else if row.connect && !row.scan}<span>Connect: {grants[row.connect]}</span>{/if}
            </div>
          {/if}
          {#if row.servicesEnabled === false || guidanceText(row) || row.optionLabel || (row.permission === 'location' && !row.optional)}
            <div
              class={`space-y-1 rounded-lg p-3 text-xs leading-5 ${row.servicesEnabled === false ? 'bg-warning/10 text-warning' : 'bg-elevated text-muted'}`}
            >
              {#if row.servicesEnabled === false}
                <p class="flex items-center gap-1.5 font-medium">
                  <Icon name="triangle-alert" class="size-3.5 shrink-0" />
                  {row.permission === 'bluetooth' ? 'Bluetooth is off.' : 'Location Services are off.'}
                </p>
              {/if}
              {#if guidanceText(row)}<p>{guidanceText(row)}</p>{/if}
              {#if row.permission === 'location' && !row.optional && row.guidance !== 'fineLocationForScan'}
                <p>This device needs precise location to find Bluetooth boards.</p>
              {/if}
              {#if row.optionLabel}<p>Choose “{row.optionLabel}”.</p>{/if}
            </div>
          {/if}
        </div>
        {#if row.action}
          <div class="border-t border-outline px-4 py-3">
            <button
              type="button"
              class={`motion-feedback flex min-h-11 w-full touch-manipulation items-center justify-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-colors disabled:opacity-50 motion-reduce:transition-none ${
                row.action === 'requestPermission'
                  ? 'bg-accent text-on-accent enabled:hover:brightness-110'
                  : 'border border-outline bg-surface text-foreground enabled:hover:bg-elevated'
              }`}
              disabled={backendState.status !== 'connected' ||
                backendState.permissionBusy ||
                backendState.snapshot?.permissions.some((p) => p.pending)}
              onpointerdown={(event) => activateFromPointer(event, () => recover(row), { scrollable: true })}
              onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => recover(row))}
            >
              {#if row.action === 'openSettings'}<Icon name="settings" class="size-4" />{/if}
              {row.action === 'requestPermission' ? 'Request access' : 'Open settings'}
            </button>
          </div>
        {/if}
      </article>
    {/each}
  </div>
  {#if backendState.status !== 'connected'}
    <div
      class="mt-3 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-outline bg-elevated p-4"
    >
      <p class="text-sm text-muted">Reconnect to update permissions.</p>
      <button
        type="button"
        class="min-h-11 touch-manipulation rounded-lg bg-accent px-4 py-2 text-sm font-medium text-on-accent disabled:opacity-50"
        onpointerdown={(event) => activateFromPointer(event, () => backendState.connect(), { scrollable: true })}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => backendState.connect())}>Reconnect</button
      >
    </div>
  {/if}
</section>
