<script lang="ts">
  import license from '../../../../../LICENSE?raw';
  import notice from '../../../../../NOTICE?raw';
  import thirdPartyNotices from '../../../../../THIRD_PARTY_NOTICES.txt?raw';
</script>

<div class="min-w-0 space-y-6 py-4">
  {#each [{ title: 'Project and artwork notices', text: notice }, { title: 'Project licence', text: license }, { title: 'Third-party notices', text: thirdPartyNotices }] as document (document.title)}
    <section aria-label={document.title} class="min-w-0 space-y-3">
      <h2 class="text-lg font-semibold">{document.title}</h2>
      <pre class="text-xs break-words whitespace-pre-wrap text-muted">{document.text}</pre>
    </section>
  {/each}
</div>
