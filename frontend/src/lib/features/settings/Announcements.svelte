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
  let openingVoiceSettings = $state(false);
  let message = $state('');
  type NumericField = 'speedThresholdKph' | 'loadThresholdPercent' | 'repeatIntervalSeconds' | 'footpadDelaySeconds';
  const numericFields: NumericField[] = [
    'speedThresholdKph',
    'loadThresholdPercent',
    'repeatIntervalSeconds',
    'footpadDelaySeconds',
  ];
  let validated = $state<Partial<Record<NumericField, boolean>>>({});
  let editVersion = 0;
  let submittedVersion = -1;
  let queuedSave: { value: AnnouncementSettings; version: number } | null = null;
  let savePromise: Promise<boolean> | null = null;
  const settings = $derived(backendState.settings);
  const audio = $derived(backendState.snapshot?.announcementAudio);
  const imperial = $derived(settings?.unitPreference === 'imperial');
  const factor = $derived(imperial ? 0.621371192237334 : 1);
  const inputClass = 'mt-1 block w-full rounded border bg-surface px-3 py-2 text-foreground';
  const fieldErrors = $derived({
    footpadDelaySeconds:
      Number.isInteger(draft.footpadDelaySeconds) && draft.footpadDelaySeconds >= 1 && draft.footpadDelaySeconds <= 5
        ? ''
        : 'Choose a whole number from 1 to 5 seconds.',
    speedThresholdKph:
      Number.isFinite(draft.speedThresholdKph) &&
      draft.speedThresholdKph * factor >= 0.001 &&
      draft.speedThresholdKph <= 200
        ? ''
        : `Enter a speed from 0.001 to ${Number((200 * factor).toFixed(3))} ${imperial ? 'mph' : 'km/h'}.`,
    loadThresholdPercent:
      Number.isFinite(draft.loadThresholdPercent) &&
      draft.loadThresholdPercent >= 0.001 &&
      draft.loadThresholdPercent <= 100
        ? ''
        : 'Enter a load from 0.001 to 100%.',
    repeatIntervalSeconds:
      Number.isInteger(draft.repeatIntervalSeconds) &&
      draft.repeatIntervalSeconds >= 1 &&
      draft.repeatIntervalSeconds <= 60
        ? ''
        : 'Enter a whole number from 1 to 60 seconds.',
  });
  const speedError = $derived(validated.speedThresholdKph ? fieldErrors.speedThresholdKph : '');
  const loadError = $derived(validated.loadThresholdPercent ? fieldErrors.loadThresholdPercent : '');
  const intervalError = $derived(validated.repeatIntervalSeconds ? fieldErrors.repeatIntervalSeconds : '');
  function commit(field: NumericField) {
    validated[field] = true;
    void save();
  }

  $effect(() => {
    if (!dirty && settings) draft = { ...settings.announcements };
  });
  function edit(patch: Partial<AnnouncementSettings>) {
    draft = { ...draft, ...patch };
    dirty = true;
    editVersion++;
    message = '';
  }
  function change(patch: Partial<AnnouncementSettings>) {
    edit(patch);
    void save();
  }
  function save(): Promise<boolean> {
    const invalid = numericFields.filter((field) => fieldErrors[field]);
    if (invalid.length) {
      for (const field of invalid) validated[field] = true;
      message = 'Correct the highlighted values to save your changes.';
      return Promise.resolve(false);
    }
    if (!dirty) return savePromise ?? Promise.resolve(true);
    // Change and blur can commit the same edit; serialize saves without losing newer edits.
    if (savePromise && submittedVersion === editVersion) return savePromise;
    submittedVersion = editVersion;
    queuedSave = { value: { ...draft }, version: editVersion };
    if (savePromise) return savePromise;
    saving = true;
    savePromise = (async () => {
      while (queuedSave) {
        const next = queuedSave;
        queuedSave = null;
        const accepted = await backendState.updateSessionSettings({ announcements: next.value });
        if (!accepted) {
          queuedSave = null;
          message = 'Could not save announcement settings. Your edits are kept; try again.';
          return false;
        }
        if (next.version === editVersion) dirty = false;
      }
      message = dirty ? '' : 'Announcement settings saved.';
      return true;
    })().finally(() => {
      saving = false;
      savePromise = null;
      submittedVersion = -1;
    });
    return savePromise;
  }
  async function openVoiceSettings() {
    if (openingVoiceSettings || saving || testing) return;
    openingVoiceSettings = true;
    message = '';
    try {
      await backendState.openVoiceSettings();
      message = 'Settings opened. Choose and fully download an offline English voice, then return here to test it.';
    } catch {
      message = 'Could not open voice settings. Open text-to-speech settings on your device and try again.';
    } finally {
      openingVoiceSettings = false;
    }
  }
  async function test(kind: 'speed' | 'load' | 'combined') {
    if (testing || openingVoiceSettings) return;
    testing = true;
    message = '';
    try {
      if (!(await save())) return;
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
      message = 'Could not play the test. Wait for any current announcement to finish and try again.';
    } finally {
      testing = false;
    }
  }
</script>

<div>
  <SectionTitle title="Ride announcements" />
  <form
    novalidate
    onsubmit={(event) => {
      event.preventDefault();
      void save();
    }}
    class="mt-3 space-y-3 rounded-lg border border-outline bg-elevated p-3 text-sm"
  >
    <p class="text-muted">
      Hear footpad contact alerts and readings above your thresholds. Load speaks only the highest of duty, motor or
      battery utilisation. Footpad alerts speak first when rules qualify together.
    </p>
    {#each ['speedEnabled', 'loadEnabled', 'footpadEnabled'] as key (key)}
      {@const field = key as 'speedEnabled' | 'loadEnabled' | 'footpadEnabled'}
      <div>
        <p>
          {field === 'speedEnabled'
            ? 'Speed announcements'
            : field === 'loadEnabled'
              ? 'Load announcements'
              : 'Footpad announcements'}
        </p>
        <SegmentedControl
          label={field === 'speedEnabled'
            ? 'Speed announcements'
            : field === 'loadEnabled'
              ? 'Load announcements'
              : 'Footpad announcements'}
          options={[
            { value: false, label: 'Off' },
            { value: true, label: 'On' },
          ]}
          value={draft[field]}
          onchange={(value) => change({ [field]: value })}
        />
      </div>
    {/each}
    <label class="block">
      Footpad delay: {draft.footpadDelaySeconds}
      {draft.footpadDelaySeconds === 1 ? 'second' : 'seconds'}
      <input
        aria-label="Footpad delay"
        aria-valuetext={`${draft.footpadDelaySeconds} seconds`}
        aria-describedby="announcement-footpad-description"
        class="mt-2 w-full touch-pan-y"
        type="range"
        min="1"
        max="5"
        step="1"
        value={draft.footpadDelaySeconds}
        oninput={(e) => edit({ footpadDelaySeconds: e.currentTarget.valueAsNumber })}
        onpointerup={(e) => {
          if (e.isPrimary && e.button === 0) commit('footpadDelaySeconds');
        }}
        onchange={() => commit('footpadDelaySeconds')}
        onblur={() => commit('footpadDelaySeconds')}
      />
      <span id="announcement-footpad-description" class="text-xs text-muted">
        Announces sustained contact loss while speed is above 5 km/h{imperial ? ' (3.1 mph)' : ''}, including in
        reverse. The delay is retained while alerts are off.
      </span>
    </label>
    <div>
      <label class="block"
        >Speed threshold ({imperial ? 'mph' : 'km/h'})
        <input
          class={`${inputClass} ${speedError ? 'border-danger' : 'border-outline'}`}
          aria-invalid={speedError ? 'true' : undefined}
          aria-describedby={speedError ? 'announcement-speed-error' : undefined}
          type="number"
          required
          min="0.001"
          max={200 * factor}
          step="any"
          value={Number((draft.speedThresholdKph * factor).toFixed(3))}
          oninput={(e) => edit({ speedThresholdKph: e.currentTarget.valueAsNumber / factor })}
          onchange={() => commit('speedThresholdKph')}
          onblur={() => commit('speedThresholdKph')}
        />
      </label>
      {#if speedError}
        <p id="announcement-speed-error" role="alert" class="mt-1 text-xs text-danger">{speedError}</p>
      {/if}
    </div>
    <div>
      <label class="block"
        >Load threshold (%)
        <input
          class={`${inputClass} ${loadError ? 'border-danger' : 'border-outline'}`}
          aria-invalid={loadError ? 'true' : undefined}
          aria-describedby={loadError ? 'announcement-load-error' : undefined}
          type="number"
          required
          min="0.001"
          max="100"
          step="any"
          value={draft.loadThresholdPercent}
          oninput={(e) => edit({ loadThresholdPercent: e.currentTarget.valueAsNumber })}
          onchange={() => commit('loadThresholdPercent')}
          onblur={() => commit('loadThresholdPercent')}
        />
      </label>
      {#if loadError}
        <p id="announcement-load-error" role="alert" class="mt-1 text-xs text-danger">{loadError}</p>
      {/if}
    </div>
    <div>
      <label class="block"
        >Repeat interval (seconds)
        <input
          class={`${inputClass} ${intervalError ? 'border-danger' : 'border-outline'}`}
          aria-invalid={intervalError ? 'true' : undefined}
          aria-describedby={intervalError ? 'announcement-interval-error' : undefined}
          type="number"
          required
          min="1"
          max="60"
          step="1"
          value={draft.repeatIntervalSeconds}
          oninput={(e) => edit({ repeatIntervalSeconds: e.currentTarget.valueAsNumber })}
          onchange={() => commit('repeatIntervalSeconds')}
          onblur={() => commit('repeatIntervalSeconds')}
        />
      </label>
      {#if intervalError}
        <p id="announcement-interval-error" role="alert" class="mt-1 text-xs text-danger">{intervalError}</p>
      {/if}
    </div>
    <label class="block"
      >Announcement volume: {draft.volumePercent}%{draft.volumePercent === 0 ? ' (muted)' : ''}
      <input
        class="mt-2 w-full touch-pan-y"
        aria-label="Announcement volume"
        type="range"
        min="0"
        max="100"
        step="1"
        value={draft.volumePercent}
        oninput={(e) => edit({ volumePercent: e.currentTarget.valueAsNumber })}
        onchange={() => void save()}
        onblur={() => void save()}
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
          onchange={(value) => change({ lowerOtherAudio: value })}
        />
        <p class="mt-1 text-xs text-muted">
          Temporarily makes music and other audio quieter during announcements. Some apps may pause instead.
        </p>
      </div>
    {/if}
    {#if audio?.voiceSettingsAvailable}
      <div>
        <button
          type="button"
          class="min-h-11 touch-manipulation rounded border border-outline px-3 py-2 disabled:opacity-50"
          disabled={openingVoiceSettings || testing || saving}
          onpointerdown={(e) => activateFromPointer(e, () => void openVoiceSettings())}
          onclick={(e) => activateFromKeyboardOrAssistiveClick(e, () => void openVoiceSettings())}
          >{openingVoiceSettings ? 'Opening…' : 'Voice settings'}</button
        >
        <p class="mt-1 text-xs text-muted">
          Choose a speech engine or preferred offline English voice in your device's text-to-speech settings. Fully
          download the voice before testing.
        </p>
      </div>
    {/if}
    {#if backendState.platform === 'ios'}
      <p class="text-xs text-muted">
        On iPhone: Settings → Accessibility → Spoken Content (Read &amp; Speak) → Voices → English. Choose a voice and
        tap its cloud icon to download it on Wi-Fi.
      </p>
    {/if}
    <div class="flex flex-wrap gap-2">
      {#each ['speed', 'load', 'combined'] as name (name)}
        {@const kind = name as 'speed' | 'load' | 'combined'}
        <button
          type="button"
          class="min-h-11 touch-manipulation rounded border border-outline px-3 py-2 disabled:opacity-50"
          disabled={testing || openingVoiceSettings}
          onpointerdown={(e) => activateFromPointer(e, () => void test(kind))}
          onclick={(e) => activateFromKeyboardOrAssistiveClick(e, () => void test(kind))}>Test {name}</button
        >
      {/each}
    </div>
    <p class="text-xs text-muted">
      {audio?.status === 'ready'
        ? 'Speech ready.'
        : audio?.status === 'initializing'
          ? 'Preparing speech…'
          : audio?.reason === 'no_offline_voice'
            ? 'No offline voice is available. Install an English voice in system speech settings, then return to the app.'
            : 'Speech is currently unavailable.'} Tests use your saved settings.
    </p>
    <p role="status" aria-live="polite">{testing ? 'Playing test…' : saving ? 'Saving…' : message}</p>
    {#if dirty && message.startsWith('Could not save')}
      <button
        type="button"
        class="min-h-11 touch-manipulation rounded border border-outline px-3 py-2 disabled:opacity-50"
        disabled={saving || openingVoiceSettings || testing}
        onpointerdown={(e) => activateFromPointer(e, () => void save())}
        onclick={(e) => activateFromKeyboardOrAssistiveClick(e, () => void save())}>Retry saving</button
      >
    {/if}
  </form>
</div>
