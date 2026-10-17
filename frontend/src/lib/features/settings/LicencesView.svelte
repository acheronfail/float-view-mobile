<script lang="ts">
  import license from '../../../../../LICENSE?raw';
  import notice from '../../../../../NOTICE?raw';
  import thirdPartyNotices from '../../../../../THIRD_PARTY_NOTICES.txt?raw';
  import LicenceSection from './LicenceSection.svelte';
  import { groupThirdPartyNotices } from './licences';

  const thirdParty = groupThirdPartyNotices(thirdPartyNotices);
</script>

<div class="min-w-0 space-y-3 py-4">
  {#each [{ title: 'Project and artwork notices', text: notice }, { title: 'Project licence', text: license }] as document (document.title)}
    <LicenceSection title={document.title}>
      <pre class="text-xs break-words whitespace-pre-wrap text-muted">{document.text}</pre>
    </LicenceSection>
  {/each}
  <LicenceSection title="Third-party notices">
    <p class="text-sm whitespace-pre-wrap text-muted">{thirdParty.introduction}</p>
    {#each thirdParty.groups as group (group.title)}
      <LicenceSection title={group.title}>
        {#each group.entries as entry (entry.title)}
          <LicenceSection title={entry.title}>
            <pre class="text-xs break-words whitespace-pre-wrap text-muted">{entry.text}</pre>
          </LicenceSection>
        {/each}
      </LicenceSection>
    {/each}
  </LicenceSection>
</div>
