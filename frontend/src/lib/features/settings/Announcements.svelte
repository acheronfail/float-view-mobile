<script lang="ts">
  import { backendState } from '$lib/backend/state.svelte';
  import { defaultAnnouncements, type AnnouncementSettings } from '$lib/backend/contract';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  import SegmentedControl from '$lib/ui/SegmentedControl.svelte';
  import SectionTitle from '$lib/ui/SectionTitle.svelte';

  let draft = $state<AnnouncementSettings>({ ...defaultAnnouncements });
  let dirty = $state(false);
  let saving = $state(false);
  let testing = $state(false);
  let message = $state('');
  let form: HTMLFormElement;
  const settings = $derived(backendState.settings);
  const audio = $derived(backendState.snapshot?.announcementAudio);
  const imperial = $derived(settings?.unitPreference === 'imperial');
  const factor = $derived(imperial ? 0.621371192237334 : 1);
  const inputClass = 'mt-1 block w-full rounded border border-outline bg-surface px-3 py-2 text-foreground';
  $effect(() => {
    if (!dirty && settings) draft = { ...settings.announcements };
  });
  function edit(patch: Partial<AnnouncementSettings>) {
    draft = { ...draft, ...patch };
    dirty = true;
    message = '';
  }
  async function save() {
    if (saving || !form.reportValidity()) return;
    saving = true;
    const accepted = await backendState.updateSessionSettings({ announcements: { ...draft } });
    saving = false;
    if (accepted) {
      dirty = false;
      message = 'Announcement settings saved.';
    } else message = 'Could not save announcement settings. Your edits are kept; try again.';
  }
  async function test(kind: 'speed' | 'load' | 'combined') {
    if (testing || saving) return;
    testing = true;
    message = '';
    try {
      const { outcome } = await backendState.testAnnouncement(kind);
      message =
        outcome === 'finished'
          ? settings?.announcements.volumePercent === 0
            ? 'Test finished silently: announcement volume is muted.'
            : 'Test finished.'
          : outcome === 'interrupted'
            ? 'Test was interrupted.'
            : 'Audio is unavailable. Try again when other audio or calls have finished.';
    } catch {
      message = 'Could not play the test. Wait for the repeat interval and try again.';
    } finally {
      testing = false;
    }
  }
</script>

<div>
  <SectionTitle title="Ride announcements" />
  <form
    bind:this={form}
    onsubmit={(event) => event.preventDefault()}
    class="mt-3 space-y-3 rounded-lg border border-outline bg-elevated p-3 text-sm"
  >
    <p class="text-muted">
      Hear readings above your thresholds. Speed and load can be enabled together; load speaks only the highest of duty,
      motor or battery utilisation.
    </p>
    {#each ['speedEnabled', 'loadEnabled'] as key (key)}
      {@const field = key as 'speedEnabled' | 'loadEnabled'}
      <div>
        <p>{field === 'speedEnabled' ? 'Speed announcements' : 'Load announcements'}</p>
        <SegmentedControl
          label={field === 'speedEnabled' ? 'Speed announcements' : 'Load announcements'}
          options={[
            { value: false, label: 'Off' },
            { value: true, label: 'On' },
          ]}
          value={draft[field]}
          disabled={saving}
          onchange={(value) => edit({ [field]: value })}
        />
      </div>
    {/each}
    <label class="block"
      >Speed threshold ({imperial ? 'mph' : 'km/h'})
      <input
        class={inputClass}
        type="number"
        required
        min="0.001"
        max={200 * factor}
        step="any"
        value={Number((draft.speedThresholdKph * factor).toFixed(3))}
        disabled={saving}
        oninput={(e) => edit({ speedThresholdKph: e.currentTarget.valueAsNumber / factor })}
      />
    </label>
    <label class="block"
      >Load threshold (%)
      <input
        class={inputClass}
        type="number"
        required
        min="0.001"
        max="100"
        step="any"
        value={draft.loadThresholdPercent}
        disabled={saving}
        oninput={(e) => edit({ loadThresholdPercent: e.currentTarget.valueAsNumber })}
      />
    </label>
    <label class="block"
      >Repeat interval (seconds)
      <input
        class={inputClass}
        type="number"
        required
        min="1"
        max="60"
        step="1"
        value={draft.repeatIntervalSeconds}
        disabled={saving}
        oninput={(e) => edit({ repeatIntervalSeconds: e.currentTarget.valueAsNumber })}
      />
    </label>
    <label class="block"
      >Announcement volume: {draft.volumePercent}%{draft.volumePercent === 0 ? ' (muted)' : ''}
      <input
        class="mt-2 w-full touch-pan-y"
        type="range"
        min="0"
        max="100"
        step="1"
        value={draft.volumePercent}
        disabled={saving}
        oninput={(e) => edit({ volumePercent: e.currentTarget.valueAsNumber })}
      />
      <span class="text-xs text-muted">Relative to your phone or computer's current volume.</span>
    </label>
    {#if audio?.loweringControl === 'adjustable'}
      <div>
        <p>Lower other audio while speaking</p>
        <SegmentedControl
          label="Lower other audio while speaking"
          options={[
            { value: false, label: 'Off' },
            { value: true, label: 'On' },
          ]}
          value={draft.lowerOtherAudio}
          disabled={saving}
          onchange={(value) => edit({ lowerOtherAudio: value })}
        />
        <p class="mt-1 text-xs text-muted">
          Temporarily makes music and other audio quieter during announcements. Some apps may pause instead.
        </p>
      </div>
    {/if}
    <button
      type="button"
      class="min-h-11 touch-manipulation rounded bg-accent px-3 py-2 text-on-accent disabled:opacity-50"
      disabled={!dirty || saving}
      onpointerdown={(e) => activateFromPointer(e, () => void save())}
      onclick={(e) => activateFromKeyboardOrAssistiveClick(e, () => void save())}
      >{saving ? 'Saving…' : 'Save announcements'}</button
    >
    <p class="text-xs text-muted">
      {audio?.status === 'ready'
        ? 'Speech ready.'
        : audio?.status === 'initializing'
          ? 'Preparing speech…'
          : audio?.reason === 'no_offline_voice'
            ? 'No offline voice is available. Install an English voice in system speech settings, then return to the app.'
            : 'Speech is currently unavailable.'} Tests use your saved settings.
    </p>
    <div class="flex flex-wrap gap-2">
      {#each ['speed', 'load', 'combined'] as name (name)}
        {@const kind = name as 'speed' | 'load' | 'combined'}
        <button
          type="button"
          class="min-h-11 touch-manipulation rounded border border-outline px-3 py-2 disabled:opacity-50"
          disabled={testing || saving}
          onpointerdown={(e) => activateFromPointer(e, () => void test(kind))}
          onclick={(e) => activateFromKeyboardOrAssistiveClick(e, () => void test(kind))}>Test {name}</button
        >
      {/each}
    </div>
    <p role="status" aria-live="polite">{testing ? 'Playing test…' : message}</p>
  </form>
</div>
