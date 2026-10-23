<script lang="ts">
  let { checked }: { checked: boolean } = $props();
</script>

<span
  aria-hidden="true"
  class="motion-feedback relative h-6 w-10 shrink-0 rounded-full border transition-colors motion-reduce:transition-none"
  class:bg-accent-fill={checked}
  class:border-accent={checked}
  class:bg-surface={!checked}
  class:border-muted={!checked}
>
  <span
    class="motion-feedback absolute top-0.5 left-0.5 h-4.5 w-4.5 rounded-full transition-transform motion-reduce:transition-none"
    class:translate-x-4={checked}
    class:bg-on-accent={checked}
    class:bg-muted={!checked}
  ></span>
</span>
