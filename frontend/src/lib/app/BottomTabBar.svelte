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
    <div class="bottom-tabs-safe-area mx-auto grid max-w-3xl grid-cols-4">
      {#each bottomTabs as tab (tab.href)}
        <a
          href={tab.href}
          aria-current={activeTab === tab ? 'page' : undefined}
          class="flex min-h-16 flex-col items-center justify-center gap-1 rounded-xl px-2 py-2 text-xs font-medium"
          class:bg-accent-soft={activeTab === tab}
          class:text-on-accent-soft={activeTab === tab}
          class:text-muted={activeTab !== tab}
        >
          {#if tab.icon === 'connect'}
            <svg
              aria-hidden="true"
              class="size-6"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
            >
              <path d="M8 12h8" />
              <path d="m13 9 3 3-3 3" />
              <path d="M5 8v8" />
            </svg>
          {:else if tab.icon === 'sessions'}
            <svg
              aria-hidden="true"
              class="size-6"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
            >
              <path d="M7 4h10" />
              <path d="M7 9h10" />
              <path d="M7 14h6" />
              <path d="M5 20h14" />
            </svg>
          {:else if tab.icon === 'boards'}
            <svg
              aria-hidden="true"
              class="size-6"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
            >
              <path d="M5 7h14" />
              <path d="M7 7 5 17h14L17 7" />
              <path d="M9 17v2" />
              <path d="M15 17v2" />
            </svg>
          {:else}
            <svg
              aria-hidden="true"
              class="size-6"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
            >
              <circle cx="12" cy="12" r="3" />
              <path
                d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.8 2.8-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.6v.2h-4V21a1.7 1.7 0 0 0-1-1.6 1.7 1.7 0 0 0-1.9.3l-.1.1L4.2 17l.1-.1a1.7 1.7 0 0 0 .3-1.9A1.7 1.7 0 0 0 3 14H2.8v-4H3a1.7 1.7 0 0 0 1.6-1 1.7 1.7 0 0 0-.3-1.9L4.2 7 7 4.2l.1.1a1.7 1.7 0 0 0 1.9.3A1.7 1.7 0 0 0 10 3v-.2h4V3a1.7 1.7 0 0 0 1 1.6 1.7 1.7 0 0 0 1.9-.3l.1-.1L19.8 7l-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.6 1h.2v4H21a1.7 1.7 0 0 0-1.6 1Z"
              />
            </svg>
          {/if}
          <span>{tab.label}</span>
        </a>
      {/each}
    </div>
  </nav>
{/if}
