<script lang="ts">
  import { getContext, onDestroy, type Snippet } from 'svelte';
  import { APP_HEADER_CONTEXT, type AppHeaderContext } from './header';

  let { owner, children }: { owner: string; children: Snippet } = $props();
  const appHeader = getContext<AppHeaderContext>(APP_HEADER_CONTEXT);

  $effect(() => {
    appHeader.set(owner, { subtitleSnippet: children });
  });

  onDestroy(() => appHeader.clear(owner));
</script>
