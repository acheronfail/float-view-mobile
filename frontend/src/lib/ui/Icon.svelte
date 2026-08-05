<script lang="ts">
  import { iconClasses, type IconName } from './icon';

  let { name, class: className = '' }: { name: IconName; class?: string } = $props();
  const classes = $derived(`${iconClasses[name]} ${className}`.trim());
</script>

<span class={classes} aria-hidden="true"></span>
