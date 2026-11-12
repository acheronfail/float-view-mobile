<script lang="ts">
  let {
    theme,
    pending = false,
    failed = false,
  }: { theme: 'light' | 'dark'; pending?: boolean; failed?: boolean } = $props();
</script>

<div class="flex flex-col items-center justify-center gap-0.5 text-muted">
  <span class={`text-base ${theme === 'dark' ? 'i-lucide-moon' : 'i-lucide-sun'}`} aria-hidden="true"></span>
  <span class="text-[10px] leading-tight font-semibold">{theme === 'dark' ? 'Dark mode' : 'Light mode'}</span>
  {#if pending || failed}<span role="status" class="text-[9px] text-warning"
      >{pending ? 'Saving…' : 'Could not save'}</span
    >{/if}
</div>
