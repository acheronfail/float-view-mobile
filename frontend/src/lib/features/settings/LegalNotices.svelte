<script lang="ts">
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  import SectionTitle from '$lib/ui/SectionTitle.svelte';
  import license from '../../../../../LICENSE?raw';
  import notice from '../../../../../NOTICE?raw';
  import thirdPartyNotices from '../../../../../THIRD_PARTY_NOTICES.txt?raw';

  let expanded = $state(false);
  const revision = __FLOAT_VIEW_BUILD_INFO__.revision;
  const source =
    revision && /^[0-9a-f]{7,40}$/.test(revision)
      ? `https://github.com/acheronfail/float-view-mobile/tree/${revision}`
      : 'https://github.com/acheronfail/float-view-mobile';
  const toggle = () => {
    expanded = !expanded;
  };
</script>

<section aria-label="Open source and licences">
  <SectionTitle title="Open source and licences" />
  <div class="mt-3 space-y-3 text-sm text-muted">
    <p>Float View is open source under AGPL-3.0-or-later.</p>
    <p>Copyright © 2026 acheronfail.</p>
    <p>
      <a
        class="touch-manipulation text-accent underline"
        href="https://github.com/acheronfail/float-view-mobile/blob/master/PRIVACY.md"
        target="_blank"
        rel="noreferrer">Privacy policy</a
      >
      ·
      <a
        class="touch-manipulation text-accent underline"
        href="https://github.com/acheronfail/float-view-mobile/issues"
        target="_blank"
        rel="noreferrer">Support</a
      >
    </p>
    <p>
      Rider artwork adapted from “onewheel” by Luis Prado, Noun Project, under CC BY 3.0. Colours, backgrounds and
      formats have been adapted for Float View.
    </p>
    <a class="inline-block touch-manipulation text-accent underline" href={source} target="_blank" rel="noreferrer">
      Source code and build instructions
    </a>
    <button
      type="button"
      class="block min-h-11 touch-manipulation rounded-lg border border-outline bg-elevated px-3 py-2 text-foreground"
      aria-expanded={expanded}
      aria-controls="license-notices"
      onpointerdown={(event) => activateFromPointer(event, toggle, { scrollable: true })}
      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, toggle)}
    >
      {expanded ? 'Hide licences and notices' : 'View licences and notices'}
    </button>
    {#if expanded}
      <div id="license-notices" class="space-y-4">
        <pre class="text-xs break-words whitespace-pre-wrap">{notice}</pre>
        <pre class="text-xs break-words whitespace-pre-wrap">{license}</pre>
        <pre class="text-xs break-words whitespace-pre-wrap">{thirdPartyNotices}</pre>
      </div>
    {/if}
  </div>
</section>
