<script lang="ts">
  import AppHeaderSubtitle from '$lib/app/AppHeaderSubtitle.svelte';
  import { formatDistanceM } from '$lib/features/measurements/format';
  import SegmentedControl from '$lib/ui/SegmentedControl.svelte';
  import type { ThemePreference, UnitPreference } from '$lib/backend/contract';
  import { backendState } from '$lib/backend/state.svelte';
  import SectionTitle from '../../ui/SectionTitle.svelte';

  import Permissions from './Permissions.svelte';

  const trackedDistanceM = $derived(
    backendState.savedDevices.reduce((total, device) => total + (device.summary.totalDistanceM ?? 0), 0)
  );

  const inputClass = 'mt-1 block w-full rounded border border-outline bg-elevated px-3 py-2 text-foreground';
  const buildInfo = __FLOAT_VIEW_BUILD_INFO__;
</script>

<AppHeaderSubtitle owner="/settings">
  {#if backendState.snapshot}
    {formatDistanceM(trackedDistanceM, backendState.settings?.unitPreference ?? 'metric')} tracked across all rides
  {:else}
    Loading ride totals…
  {/if}
</AppHeaderSubtitle>

<section aria-label="Settings" class="space-y-4">
  {#if backendState.settings}
    <div>
      <SectionTitle title="Appearance" />
      <div class="mt-3 space-y-1 text-sm text-muted">
        <p>Theme</p>
        <SegmentedControl
          label="Theme"
          options={[
            { value: 'system', label: 'System' },
            { value: 'light', label: 'Light' },
            { value: 'dark', label: 'Dark' },
          ]}
          value={backendState.settings.themePreference}
          onchange={(themePreference: ThemePreference) => backendState.updateSessionSettings({ themePreference })}
        />
      </div>
      <div class="mt-3 space-y-1 text-sm text-muted">
        <p>Units</p>
        <SegmentedControl
          label="Units"
          options={[
            { value: 'metric', label: 'Metric' },
            { value: 'imperial', label: 'Imperial' },
          ]}
          value={backendState.settings.unitPreference}
          onchange={(unitPreference: UnitPreference) => backendState.updateSessionSettings({ unitPreference })}
        />
      </div>
    </div>

    <div>
      <SectionTitle title="Session settings" />
      <div class="mt-3 space-y-3 rounded-lg border border-outline bg-elevated p-3">
        <div class="space-y-2 text-sm text-muted">
          <p>Record ride location</p>
          <SegmentedControl
            label="Record ride location"
            options={[
              { value: false, label: 'Off' },
              { value: true, label: 'On' },
            ]}
            value={backendState.settings.locationRecordingEnabled}
            onchange={(value) => backendState.updateSessionSettings({ locationRecordingEnabled: value })}
          />
        </div>
        <div class="space-y-2 text-sm text-muted">
          <p>Include GPS columns in CSV exports</p>
          <SegmentedControl
            label="Include GPS columns in CSV exports"
            options={[
              { value: false, label: 'Off' },
              { value: true, label: 'On' },
            ]}
            value={backendState.settings.includeGpsLocationWhenExporting}
            onchange={(value) => backendState.updateSessionSettings({ includeGpsLocationWhenExporting: value })}
          />
        </div>
        <p class="text-xs text-muted">Location capture is native-owned and stops outside active riding.</p>
      </div>
      <label class="mt-3 block text-sm text-muted">
        Minimum ride distance (meters)
        <input
          class={inputClass}
          type="number"
          min="0"
          max="1000000"
          step="10"
          value={backendState.settings.minimumSessionDistance}
          onchange={(event) =>
            backendState.updateSessionSettings({
              minimumSessionDistance: Number(event.currentTarget.value),
            })}
        />
      </label>
      <label class="mt-3 block text-sm text-muted">
        History gap threshold (seconds)
        <input
          class={inputClass}
          type="number"
          min="0.1"
          max="3600"
          step="0.1"
          placeholder="Automatic"
          value={backendState.settings.historyGapThresholdSeconds ?? ''}
          aria-describedby="history-gap-help"
          onchange={(event) => {
            const value = event.currentTarget.value.trim();
            const seconds = Number(value);
            backendState.updateSessionSettings({
              historyGapThresholdSeconds:
                value === '' || !Number.isFinite(seconds) ? null : Math.max(0.1, Math.min(3600, seconds)),
            });
          }}
        />
        <span id="history-gap-help" class="mt-1 block text-xs"
          >Automatic uses the captured persistence and retained-resolution policy for each session.</span
        >
      </label>
    </div>

    <Permissions />

    <div>
      <SectionTitle title="Connection" />
      <label class="text-sm text-muted"
        >Reconnect window (minutes)
        <input
          class={inputClass}
          type="number"
          min="0"
          max="60"
          step="1"
          value={backendState.settings.reconnectDurationMs / 60_000}
          onchange={(event) =>
            backendState.updateSessionSettings({
              reconnectDurationMs: Math.round(Number(event.currentTarget.value) * 60_000),
            })}
        />
      </label>
    </div>

    <div>
      <SectionTitle title="Logging" />
      <p class="mt-1 text-xs text-muted">These apply natively, including while the frontend is not running.</p>
      <div class="mt-3 grid grid-cols-1 gap-3 sm:grid-cols-2">
        <label class="text-sm text-muted"
          >Idle samples / second
          <input
            class={inputClass}
            type="number"
            min="0.0167"
            max="10"
            step="0.1"
            value={1000 / backendState.settings.idlePersistIntervalMs}
            onchange={(event) =>
              backendState.updateSessionSettings({
                idlePersistIntervalMs: Math.round(1000 / Math.max(0.0167, Number(event.currentTarget.value))),
              })}
          />
        </label>
        <label class="text-sm text-muted"
          >Riding samples / second
          <input
            class={inputClass}
            type="number"
            min="0.1"
            max="20"
            step="1"
            value={1000 / backendState.settings.ridingPersistIntervalMs}
            onchange={(event) =>
              backendState.updateSessionSettings({
                ridingPersistIntervalMs: Math.round(1000 / Math.max(0.1, Number(event.currentTarget.value))),
              })}
          />
        </label>
        <label class="text-sm text-muted"
          >Ride-state probes / second
          <input
            class={inputClass}
            type="number"
            min="0.1"
            max="10"
            step="0.5"
            value={1000 / backendState.settings.idleProbeIntervalMs}
            onchange={(event) =>
              backendState.updateSessionSettings({
                idleProbeIntervalMs: Math.round(1000 / Math.max(0.1, Number(event.currentTarget.value))),
              })}
          />
        </label>
        <label class="text-sm text-muted"
          >Post-ride high-rate tail (seconds)
          <input
            class={inputClass}
            type="number"
            min="0"
            max="300"
            step="1"
            value={backendState.settings.postRideTailMs / 1000}
            onchange={(event) =>
              backendState.updateSessionSettings({
                postRideTailMs: Math.round(Number(event.currentTarget.value) * 1000),
              })}
          />
        </label>
      </div>
    </div>
  {:else}
    <div class="rounded-2xl border border-outline bg-surface p-5 text-muted">Loading settings…</div>
  {/if}
  {#if backendState.error}<p role="alert" class="text-warning">{backendState.error}</p>{/if}

  <footer aria-label="Application version" class="border-t border-outline pt-4 pb-2 text-center text-xs text-muted">
    <p>Float View · {buildInfo.buildNumber ? `Build ${buildInfo.buildNumber}` : 'Development build'}</p>
    {#if buildInfo.revision}
      <p class="mt-1 font-mono">{buildInfo.revision}</p>
    {/if}
  </footer>
</section>
