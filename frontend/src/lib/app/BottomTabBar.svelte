<script lang="ts">
  import { bottomTabForRoute, bottomTabs } from './navigation';

  let { routeId }: { routeId: string | null } = $props();
  const activeTab = $derived(bottomTabForRoute(routeId));
</script>

{#if activeTab}
  <nav
    aria-label="Primary"
    data-app-navigation="bottom-tabs"
    class="fixed inset-x-0 bottom-0 z-10 border-t border-outline bg-surface/95 backdrop-blur"
  >
    <div class="bottom-tabs-safe-area mx-auto grid max-w-3xl grid-cols-3">
      {#each bottomTabs as tab (tab.href)}
        <a
          href={tab.href}
          aria-current={activeTab === tab ? 'page' : undefined}
          class="my-2 flex min-h-14 flex-col items-center justify-center gap-1 rounded-xl px-2 text-xs font-medium"
          class:bg-accent-soft={activeTab === tab}
          class:text-on-accent-soft={activeTab === tab}
          class:text-muted={activeTab !== tab}
          >
          {#if tab.icon === 'connect'}
            <svg aria-hidden="true" class="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M12 3v18l5-5-5-5 5-5-5-3M5 7l14 10M5 17 19 7" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          {:else if tab.icon === 'sessions'}
            <svg aria-hidden="true" class="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M4 19V5M4 19h16M8 16l3-4 3 2 5-7" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          {:else}
            <svg aria-hidden="true" class="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="3.5" />
              <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-1.7 1.7-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.6v.1h-2.4v-.1a1.7 1.7 0 0 0-1-1.6 1.7 1.7 0 0 0-1.9.3l-.1.1-1.7-1.7.1-.1a1.7 1.7 0 0 0 .3-1.9 1.7 1.7 0 0 0-1.6-1H6.7v-2.4h.1a1.7 1.7 0 0 0 1.6-1 1.7 1.7 0 0 0-.3-1.9L8 8.6l1.7-1.7.1.1a1.7 1.7 0 0 0 1.9.3 1.7 1.7 0 0 0 1-1.6v-.1h2.4v.1a1.7 1.7 0 0 0 1 1.6 1.7 1.7 0 0 0 1.9-.3l.1-.1 1.7 1.7-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.6 1h.1V14h-.1a1.7 1.7 0 0 0-1.6 1Z" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          {/if}
          <span>{tab.label}</span>
        </a>
      {/each}
    </div>
  </nav>
{/if}
