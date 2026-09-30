<script lang="ts">
  import { backendState } from '$lib/backend/state.svelte';
  import type { PermissionGuidance, PermissionRow } from '$lib/backend/contract';
  import SectionTitle from '$lib/ui/SectionTitle.svelte';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';

  const names = {
    bluetooth: 'Bluetooth / Nearby devices',
    location: 'Location',
    backgroundLocation: 'Background location',
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
    backgroundLocation: 'Record ride routes while the app is in the background or the phone is locked.',
  };
  const statuses = {
    notDetermined: 'Not requested',
    granted: 'Allowed',
    limited: 'Limited access',
    denied: 'Not allowed',
    restricted: 'Restricted',
    unavailable: 'Checking / unavailable',
  };
  const guidance: Record<PermissionGuidance, string> = {
    none: '',
    checking: 'Current access could not be established. Return to the app to refresh.',
    restricted: 'Access is restricted by the operating system or device policy.',
    unsupportedHardware: 'Bluetooth hardware is unavailable on this device.',
    servicesDisabled: 'The system service is off. Enable it in system settings; your permission grant is unchanged.',
    foregroundRequired: 'Allow Location first. Background access is a separate, optional step.',
    rationale: 'The system will explain why this access is needed before requesting it again.',
    iosSettings: 'In Settings, open Float View and select Location or enable Bluetooth.',
    appSettings: 'Open the app’s system settings, then Permissions, and enable this access.',
    macBluetooth: 'In System Settings, open Privacy & Security → Bluetooth and enable Float View.',
    macLocation: 'In System Settings, open Privacy & Security → Location Services and enable Float View.',
    backgroundSettings:
      'In app settings, open Permissions → Location and allow background access. You can cancel before opening settings.',
    alwaysAccess:
      'While Using allows foreground recording. Choose Always to record rides that begin while the phone is locked.',
    fineLocationForScan: 'This Android version also needs precise location to scan for Bluetooth boards.',
  };
  function recover(row: PermissionRow) {
    return backendState.recoverPermission(row);
  }
</script>

<section aria-label="Permissions">
  <SectionTitle title="Permissions" />
  <p class="mt-2 text-sm text-muted">
    Access is managed by your operating system. These actions do not enable recording or connect to a board.
  </p>
  {#if backendState.permissionError}<p role="alert" class="mt-2 text-sm text-danger">
      {backendState.permissionError} Use the system settings instructions below if navigation failed.
    </p>{/if}
  <div class="mt-3 space-y-3">
    {#each backendState.snapshot?.permissions ?? [] as row (row.permission)}
      <div class="rounded-lg border border-outline bg-elevated p-3" data-permission={row.permission}>
        <h3 class="font-medium">{names[row.permission]}</h3>
        <p class="text-sm">{statuses[row.status]}{row.pending ? ' — Waiting for the system…' : ''}</p>
        <p class="mt-1 text-sm text-muted">{purposes[row.permission]}</p>
        {#if row.permission === 'location' && !row.optional}<p class="text-xs text-muted">
            This device also requires precise location to scan for Bluetooth boards.
          </p>{/if}
        {#if row.optional}<p class="text-xs text-muted">
            Optional for board-only use. Ride location recording is {backendState.settings?.locationRecordingEnabled
              ? 'on'
              : 'off'}.
          </p>{/if}
        {#if row.accuracy !== 'unknown'}<p class="text-sm">
            {row.accuracy === 'precise' ? 'Precise' : 'Approximate'} location
          </p>{/if}
        {#if row.scope !== 'unknown'}<p class="text-sm">
            {row.scope === 'always' ? 'Always' : 'While using the app'}
          </p>{/if}
        {#if row.scan}<p class="text-sm">
            Scan: {grants[row.scan]}. Connect: {row.connect ? grants[row.connect] : 'Unknown'}.
          </p>{/if}
        {#if row.servicesEnabled === false}<p class="text-sm">System service is off.</p>{/if}
        {#if guidance[row.guidance]}<p class="mt-1 text-sm text-muted">{guidance[row.guidance]}</p>{/if}
        {#if row.optionLabel}<p class="text-sm">Choose “{row.optionLabel}”.</p>{/if}
        {#if row.action}
          <button
            type="button"
            class="mt-2 touch-manipulation rounded border border-outline px-3 py-2 text-sm disabled:opacity-50"
            disabled={backendState.status !== 'connected' ||
              backendState.permissionBusy ||
              backendState.snapshot?.permissions.some((p) => p.pending)}
            onpointerdown={(event) => activateFromPointer(event, () => recover(row))}
            onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => recover(row))}
          >
            {row.action === 'requestPermission' ? 'Request access' : 'Open settings'}
          </button>
        {/if}
      </div>
    {/each}
  </div>
  {#if backendState.status !== 'connected'}
    <p class="mt-2 text-sm text-muted">Reconnect to update permissions or request access.</p>
    <button
      type="button"
      class="mt-2 touch-manipulation rounded border border-outline px-3 py-2 text-sm"
      onpointerdown={(event) => activateFromPointer(event, () => backendState.connect())}
      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => backendState.connect())}>Reconnect</button
    >
  {/if}
</section>
