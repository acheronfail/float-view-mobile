<script lang="ts" module>
  export type MenuItemAppearance = 'normal' | 'warning' | 'destructive';
</script>

<script lang="ts">
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer, type Activation } from './activation';
  import Icon from './Icon.svelte';
  import ToggleIndicator from './ToggleIndicator.svelte';

  const statusId = $props.id();

  let {
    label,
    onactivate,
    appearance = 'normal',
    disabled = false,
    checked,
    pending = false,
  }: {
    label: string;
    onactivate: Activation;
    appearance?: MenuItemAppearance;
    disabled?: boolean;
    checked?: boolean | null;
    pending?: boolean;
  } = $props();
</script>

<button
  type="button"
  class="flex w-full touch-manipulation flex-row items-center gap-2 px-4 py-3 text-left hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:bg-transparent"
  class:text-danger={appearance === 'destructive'}
  class:text-warning={appearance === 'warning'}
  disabled={disabled || pending || checked === null}
  role={typeof checked === 'boolean' ? 'switch' : undefined}
  aria-checked={typeof checked === 'boolean' ? checked : undefined}
  aria-label={label}
  aria-describedby={checked === null ? statusId : undefined}
  aria-busy={pending || undefined}
  onpointerdown={(event) => activateFromPointer(event, onactivate)}
  onclick={(event) => activateFromKeyboardOrAssistiveClick(event, onactivate)}
>
  {#if appearance === 'warning'}<Icon name="triangle-alert" class="mr-1 text-xl" />{/if}
  <span class="flex-1 whitespace-nowrap">{label}</span>
  {#if checked === null}
    <span id={statusId} role="status" class="text-xs text-muted">Unknown</span>
  {/if}
  {#if typeof checked === 'boolean'}
    <ToggleIndicator {checked} />
  {/if}
</button>
