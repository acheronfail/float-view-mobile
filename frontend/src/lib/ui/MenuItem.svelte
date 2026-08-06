<script lang="ts" module>
  export type MenuItemAppearance = 'normal' | 'warning' | 'destructive';
</script>

<script lang="ts">
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer, type Activation } from './activation';
  import Icon from './Icon.svelte';

  let {
    label,
    onactivate,
    appearance = 'normal',
    disabled = false,
  }: {
    label: string;
    onactivate: Activation;
    appearance?: MenuItemAppearance;
    disabled?: boolean;
  } = $props();
</script>

<button
  type="button"
  class="flex w-full touch-manipulation flex-row items-center gap-2 px-4 py-3 text-left hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:bg-transparent"
  class:text-danger={appearance === 'destructive'}
  class:text-warning={appearance === 'warning'}
  {disabled}
  onpointerdown={(event) => activateFromPointer(event, onactivate)}
  onclick={(event) => activateFromKeyboardOrAssistiveClick(event, onactivate)}
>
  {#if appearance === 'warning'}<Icon name="triangle-alert" class="mr-1 text-xl" />{/if}
  {label}
</button>
