<script lang="ts">
  import { backendState } from '$lib/backend/state.svelte';
  import AppDataTransfer from './AppDataTransfer.svelte';
  import OnOffSetting from './OnOffSetting.svelte';
  import SectionTitle from '$lib/ui/SectionTitle.svelte';
</script>

<AppDataTransfer>
  {#if backendState.settings}
    <section class="space-y-3">
      <SectionTitle title="CSV exports" />
      <OnOffSetting
        label="Include GPS columns in CSV exports"
        checked={backendState.settings.includeGpsLocationWhenExporting}
        onchange={(includeGpsLocationWhenExporting) =>
          backendState.updateSessionSettings({ includeGpsLocationWhenExporting })}
      />
      <p class="text-xs text-muted">
        Controls whether session CSV exports include recorded locations. App backups always include recorded locations.
      </p>
    </section>
  {/if}
</AppDataTransfer>
