<script lang="ts">
  import SectionTitle from '$lib/ui/SectionTitle.svelte';
  import LegalNotices from './LegalNotices.svelte';
  const buildInfo = __FLOAT_VIEW_BUILD_INFO__;
</script>

<section class="space-y-2">
  <SectionTitle title="Float View" />
  <p class="text-sm text-muted">{buildInfo.version ?? 'Development build'}</p>
  {#if buildInfo.showRevision && buildInfo.revision}<p class="font-mono text-xs break-all text-muted">
      {buildInfo.revision}
    </p>{/if}
</section>
<LegalNotices />
