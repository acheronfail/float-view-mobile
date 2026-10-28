<script lang="ts">
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  import { backendState } from '$lib/backend/state.svelte';
  let edited = $state<string | null>(null);
  let error = $state('');
  let saving = $state(false);
  let retry = $state(false);
  async function save() {
    const value = Number(edited ?? backendState.settings?.rideResumeWindowMinutes);
    if (!Number.isInteger(value) || value < 1 || value > 1440) {
      error = 'Enter a whole number from 1 to 1,440 minutes.';
      return;
    }
    if (saving) return;
    saving = true;
    retry = false;
    const submitted = edited;
    const ok = await backendState.updateSessionSettings({ rideResumeWindowMinutes: value });
    saving = false;
    if (ok && submitted === edited) {
      edited = null;
      error = '';
    } else if (!ok) {
      error = 'Could not save how long to keep rides open.';
      retry = true;
    }
    if (ok && submitted !== edited && edited !== null) void save();
  }
</script>

<label class="mt-3 block text-sm text-muted">
  Keep ride open for (minutes)
  <input
    class="mt-1 block w-full rounded border border-outline bg-elevated px-3 py-2 text-foreground"
    type="number"
    inputmode="numeric"
    min="1"
    max="1440"
    step="1"
    value={edited ?? backendState.settings?.rideResumeWindowMinutes ?? 60}
    aria-invalid={!!error}
    aria-describedby="ride-resume-help ride-resume-error"
    oninput={(event) => {
      edited = event.currentTarget.value;
      error = '';
    }}
    onchange={() => void save()}
  />
</label>
<p id="ride-resume-help" class="mt-1 text-xs text-muted">
  Keep the same ride across disconnects and charging breaks. 1–1,440 minutes (24 hours). Changes apply to your next
  break.
</p>
<p id="ride-resume-error" role={error ? 'alert' : undefined} class="mt-1 text-sm text-danger">{error}</p>

{#if retry}
  <button
    type="button"
    class="min-h-11 touch-manipulation rounded border border-outline px-3 text-sm"
    disabled={saving}
    onpointerdown={(event) => activateFromPointer(event, () => void save())}
    onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => void save())}>Retry saving</button
  >
{/if}
