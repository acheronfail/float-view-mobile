<script lang="ts" module>
  export type MenuItemAppearance = 'normal' | 'warning' | 'destructive';
</script>

<script lang="ts">
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer, type Activation } from './activation';

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
  class="block w-full touch-manipulation px-4 py-3 text-left hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:bg-transparent"
  class:text-danger={appearance === 'destructive'}
  class:text-warning={appearance === 'warning'}
  {disabled}
  onpointerdown={(event) => activateFromPointer(event, onactivate)}
  onclick={(event) => activateFromKeyboardOrAssistiveClick(event, onactivate)}
>
  {#if appearance === 'warning'}⚠️️{/if}
  {label}
</button>
