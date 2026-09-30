<script lang="ts">
  import Icon from '$lib/ui/Icon.svelte';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';

  let { onconfigure }: { onconfigure: (trigger: HTMLButtonElement) => void } = $props();
</script>

<button
  type="button"
  class="inline-flex size-9 shrink-0 touch-manipulation items-center justify-center rounded-full text-warning"
  aria-label="Battery setup required"
  title="Configure battery for an estimated percentage"
  onpointerdown={(event) => {
    const trigger = event.currentTarget;
    activateFromPointer(event, () => onconfigure(trigger));
  }}
  onclick={(event) => {
    const trigger = event.currentTarget;
    activateFromKeyboardOrAssistiveClick(event, () => onconfigure(trigger));
  }}
>
  <Icon name="triangle-alert" class="text-base" />
</button>
