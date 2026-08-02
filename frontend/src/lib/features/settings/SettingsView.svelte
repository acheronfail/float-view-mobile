<script lang="ts">
  import type { ThemePreference, UnitPreference } from '$lib/backend/contract';
  import { backendState } from '$lib/backend/state.svelte';

  const inputClass = 'mt-1 block w-full rounded border border-outline bg-elevated px-3 py-2 text-foreground';
</script>

<section aria-labelledby="settings-heading" class="space-y-4">
  <div class="rounded-2xl border border-outline bg-surface p-5">
    <h2 id="settings-heading" class="text-xl font-medium">Settings</h2>
    <p class="mt-1 text-sm text-muted">Options that apply across all boards and sessions.</p>
  </div>

  {#if backendState.settings}
    <div class="rounded-2xl border border-outline bg-surface p-5">
      <h3 class="font-medium">Appearance</h3>
      <label class="mt-3 block text-sm text-muted">
        Theme
        <select
          class={inputClass}
          value={backendState.settings.themePreference}
          onchange={(event) =>
            backendState.updateSessionSettings({
              themePreference: event.currentTarget.value as ThemePreference,
            })}
        >
          <option value="system">System</option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
        </select>
      </label>
      <label class="mt-3 block text-sm text-muted">
        Units
        <select
          class={inputClass}
          value={backendState.settings.unitPreference}
          onchange={(event) =>
            backendState.updateSessionSettings({
              unitPreference: event.currentTarget.value as UnitPreference,
            })}
        >
          <option value="metric">Metric</option>
          <option value="imperial">Imperial</option>
        </select>
      </label>
    </div>

    <div class="rounded-2xl border border-outline bg-surface p-5">
      <h3 class="font-medium">Sessions</h3>
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
      <label class="mt-3 flex items-center gap-3 text-sm">
        <input
          type="checkbox"
          checked={backendState.settings.automaticSessionLinkingEnabled}
          onchange={(event) =>
            backendState.updateSessionSettings({
              automaticSessionLinkingEnabled: event.currentTarget.checked,
            })}
        />
        <span>Link same-board rides across charging breaks</span>
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
      <label class="mt-3 block text-sm text-muted">
        Maximum break (hours)
        <input
          class={inputClass}
          type="number"
          min="0.0167"
          max="24"
          step="0.5"
          value={backendState.settings.sessionLinkWindowMs / 3_600_000}
          onchange={(event) =>
            backendState.updateSessionSettings({
              sessionLinkWindowMs: Math.round(Number(event.currentTarget.value) * 3_600_000),
            })}
        />
      </label>
    </div>

    <div class="rounded-2xl border border-outline bg-surface p-5">
      <h3 class="font-medium">Logging and recovery</h3>
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
    </div>
  {:else}
    <div class="rounded-2xl border border-outline bg-surface p-5 text-muted">Loading settings…</div>
  {/if}
  {#if backendState.error}<p role="alert" class="text-warning">{backendState.error}</p>{/if}
</section>
