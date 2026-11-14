<script lang="ts">
  import { formatSpeedKph } from '../measurements/format';
  import { backendState } from '$lib/backend/state.svelte';
  import { emptySpeedCalibration, type SpeedCalibrationMeasurementStatus } from '$lib/backend/contract';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';

  const presentation: Record<SpeedCalibrationMeasurementStatus, string> = {
    inactive: 'Preparing',
    waitingGps: 'Waiting for GPS',
    waitingBoard: 'Waiting for board',
    lowSpeed: 'Speed too low',
    gpsUncertain: 'GPS speed uncertain',
    speedChanging: 'Keep speed steady',
    speedMismatch: 'Speed readings differ',
    collecting: 'Measuring',
    needsMoreMeasurements: 'Measuring',
    inconsistentMeasurements: 'Measuring',
    ready: 'Ready',
  };
  const calibration = $derived(backendState.snapshot?.speedCalibration ?? emptySpeedCalibration);
  const location = $derived(backendState.snapshot?.location);
  const locationAccess = $derived(backendState.snapshot?.permissions.find((p) => p.permission === 'location'));
  const permitted = $derived(
    location?.status === 'authorized' ||
      location?.status === 'foregroundOnly' ||
      (locationAccess?.status === 'granted' && locationAccess.servicesEnabled !== false)
  );
  const connected = $derived(backendState.snapshot?.activeConnection?.state === 'ready');
  const collecting = $derived(calibration.status === 'collecting');
  const recommended = $derived(calibration.recommendedDiameterMm);
  const units = $derived(backendState.snapshot?.settings?.unitPreference ?? 'metric');
  const minimumSpeed = $derived(formatSpeedKph(calibration.minimumSpeedKph, units));
  const sampleSpeed = $derived(backendState.snapshot?.activeConnection?.latestSample?.speedKph);
  const speed = $derived(
    connected && calibration.measurementStatus !== 'waitingBoard' && sampleSpeed != null && Number.isFinite(sampleSpeed)
      ? Math.abs(sampleSpeed) * (units === 'metric' ? 1 : 0.621371)
      : null
  );
  let displayedStatus = $state<SpeedCalibrationMeasurementStatus | null>(null);
  const desiredStatus = $derived(calibration.measurementStatus);
  const shownStatus = $derived(displayedStatus ?? desiredStatus);
  const measuring = $derived(
    shownStatus === 'collecting' ||
      shownStatus === 'needsMoreMeasurements' ||
      shownStatus === 'inconsistentMeasurements'
  );
  const message = $derived(presentation[shownStatus]);
  // Preserve glanceable feedback through brief rejected fixes. Collection itself
  // responds immediately in the service; this delay only stabilizes presentation.
  $effect(() => {
    const next = desiredStatus;
    if (displayedStatus === null || !collecting || next === 'collecting' || next === 'needsMoreMeasurements') {
      displayedStatus = next;
      return;
    }
    if (displayedStatus === next) return;
    const timer = setTimeout(() => {
      displayedStatus = next;
    }, 1200);
    return () => clearTimeout(timer);
  });
  let diameter = $state<number | undefined>(undefined);
  let working = $state(false);
  let error = $state<string | null>(null);
  const starting = $derived(calibration.currentDiameterMm ?? diameter);
  const validDiameter = $derived(
    typeof starting === 'number' && Number.isFinite(starting) && starting >= 100 && starting <= 600
  );

  async function action(kind: 'start' | 'cancel' | 'apply') {
    if (working) return;
    working = true;
    error = null;
    try {
      if (!(await backendState.speedCalibration(kind, starting))) {
        error =
          kind === 'apply'
            ? 'The board update could not be verified.'
            : kind === 'start' && calibration.currentDiameterMm === null
              ? 'Enter the current wheel diameter from VESC Tool, then start again.'
              : (backendState.error ?? 'Calibration could not be updated. Please try again.');
      }
    } finally {
      working = false;
    }
  }
  async function permission() {
    if (working) return;
    working = true;
    try {
      await backendState.requestLocationPermission();
    } finally {
      working = false;
    }
  }
</script>

<div class="mx-auto flex w-full max-w-lg flex-col gap-6 p-4 pb-8" class:h-full={collecting}>
  {#if calibration.status === 'applied'}
    <section class="rounded-2xl border border-accent/30 bg-accent/10 p-5" aria-live="polite">
      <h2 class="text-lg font-semibold">Wheel diameter updated</h2>
      <p class="mt-2 text-3xl font-semibold tabular-nums">{recommended?.toFixed(1)} <span class="text-lg">mm</span></p>
    </section>
  {:else if recommended !== null && !collecting}
    <section class="rounded-2xl border border-accent/30 bg-accent/10 p-5" aria-live="polite">
      <h2 class="text-sm font-medium text-muted">Recommended wheel diameter</h2>
      <p class="mt-2 text-4xl font-semibold tabular-nums">{recommended.toFixed(1)} <span class="text-xl">mm</span></p>
      <p class="mt-2 text-sm text-muted">Starting setting: {calibration.startingDiameterMm?.toFixed(1)} mm</p>
    </section>
    <div class="flex flex-col gap-3">
      {#if calibration.applyBlockReason !== 'unsupportedSetup' && calibration.applyBlockReason !== 'connectionChanged'}
        <button
          type="button"
          class="touch-manipulation rounded-xl bg-accent-fill px-4 py-3 font-semibold text-on-accent disabled:opacity-50"
          disabled={working || !calibration.applyAvailable}
          onpointerdown={(e) => activateFromPointer(e, () => action('apply'))}
          onclick={(e) => activateFromKeyboardOrAssistiveClick(e, () => action('apply'))}
        >
          {working || calibration.applyBlockReason === 'applying' ? 'Applying…' : `Apply ${recommended.toFixed(1)} mm`}
        </button>
        {#if calibration.applyBlockReason === 'stopRiding'}<p class="text-sm text-muted">
            Apply is available when the board is idle.
          </p>{:else if calibration.applyBlockReason === 'busy'}<p class="text-sm text-muted">
            Wait for the other board update to finish.
          </p>{:else if calibration.applyBlockReason === 'boardNotReady'}<p class="text-sm text-muted">
            The board isn’t ready to update yet. Your recommendation is still available.
          </p>{/if}
      {:else}
        <p class="text-sm text-muted">Apply is unavailable for this setup.</p>
      {/if}
      <p class="text-sm text-muted">
        In VESC Tool, open Motor Settings → Additional Info → Setup. Set Wheel Diameter to <strong
          class="text-foreground">{recommended.toFixed(1)} mm</strong
        >, then write motor configuration.
      </p>
    </div>
  {:else if collecting}
    <section class="flex flex-1 flex-col justify-between gap-8" aria-label="Speed comparison">
      <div class="flex flex-1 flex-col items-center justify-center gap-8 py-6">
        <div
          class={`w-full rounded-2xl border px-4 py-6 text-center ${measuring ? 'border-accent/40 bg-accent/10' : 'border-warning/40 bg-warning/10'}`}
          role="status"
          aria-live="polite"
          aria-atomic="true"
        >
          <h2 class="text-4xl leading-tight font-semibold">{message}</h2>
          {#if shownStatus === 'lowSpeed'}<p class="mt-3 text-lg">Minimum {minimumSpeed}</p>{/if}
        </div>
        <div class="text-center">
          <p class="text-7xl font-semibold tracking-tight tabular-nums" data-calibration-speed>
            {speed === null ? '—' : speed.toFixed(1)}
          </p>
          <p class="mt-2 text-base text-muted">Board speed · {units === 'metric' ? 'km/h' : 'mph'}</p>
        </div>
        {#if location?.backgroundCapture !== 'available'}
          <p class="text-sm text-muted">Measurements pause in the background.</p>
        {/if}
      </div>
      <button
        type="button"
        class="touch-manipulation rounded-xl border border-outline px-4 py-3 disabled:opacity-50"
        disabled={working}
        onpointerdown={(e) => activateFromPointer(e, () => action('cancel'))}
        onclick={(e) => activateFromKeyboardOrAssistiveClick(e, () => action('cancel'))}>Cancel</button
      >
    </section>
  {:else}
    <section class="flex flex-col gap-4">
      <h2 class="text-lg font-semibold">Match board speed to GPS</h2>
      <p class="text-sm text-muted">
        We’ll compare GPS and board speed to recommend a wheel diameter. Calibration finishes automatically.
      </p>
      {#if calibration.status === 'insufficient'}
        <p role="status" class="rounded-xl border border-outline p-3 text-sm">
          Calibration ended without a reliable diameter estimate.
        </p>
      {/if}
      {#if calibration.currentDiameterMm !== null}
        <p class="text-sm">Current wheel diameter: <strong>{calibration.currentDiameterMm.toFixed(1)} mm</strong></p>
      {:else}
        <label class="flex flex-col gap-2 text-sm" for="calibration-diameter"
          >Current wheel diameter (mm)
          <input
            id="calibration-diameter"
            type="number"
            min="100"
            max="600"
            step="0.1"
            inputmode="decimal"
            bind:value={diameter}
            class="rounded-xl border border-outline bg-surface px-3 py-3 text-foreground"
          />
          <span class="text-muted"
            >Enter the value currently set in VESC Tool → Motor Settings → Additional Info → Setup.</span
          >
        </label>
      {/if}
      {#if !permitted}
        <p class="text-sm text-muted">Location access is required to measure GPS speed.</p>
        <button
          type="button"
          class="touch-manipulation rounded-xl border border-outline px-4 py-3 disabled:opacity-50"
          disabled={working || !location?.permissionAction}
          onpointerdown={(e) => activateFromPointer(e, permission)}
          onclick={(e) => activateFromKeyboardOrAssistiveClick(e, permission)}
        >
          {location?.permissionAction === 'openSettings' ? 'Open location settings' : 'Allow location'}
        </button>
      {/if}
      <button
        type="button"
        class="touch-manipulation rounded-xl bg-accent-fill px-4 py-3 font-semibold text-on-accent disabled:opacity-50"
        disabled={working || !connected || !permitted || !validDiameter}
        onpointerdown={(e) => activateFromPointer(e, () => action('start'))}
        onclick={(e) => activateFromKeyboardOrAssistiveClick(e, () => action('start'))}>Start calibration</button
      >
    </section>
  {/if}
  {#if !connected}<p role="status" class="text-sm text-muted">Reconnect the board to calibrate.</p>{/if}
  {#if error}<p role="alert" class="text-sm text-warning">{error}</p>{/if}
  {#if !collecting && (recommended !== null || calibration.status === 'applied')}
    <button
      type="button"
      class="touch-manipulation rounded-xl border border-outline px-4 py-3 disabled:opacity-50"
      disabled={working}
      onpointerdown={(e) => activateFromPointer(e, () => action('cancel'))}
      onclick={(e) => activateFromKeyboardOrAssistiveClick(e, () => action('cancel'))}>Calibrate again</button
    >
  {/if}
</div>
