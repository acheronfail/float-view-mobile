<script lang="ts" module>
  export type MenuItemAppearance = 'normal' | 'warning' | 'danger' | 'destructive';
</script>

<script lang="ts">
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer, type Activation } from './activation';
  import Icon from './Icon.svelte';
  import type { IconName } from './icon';
  import ToggleIndicator from './ToggleIndicator.svelte';

  const statusId = $props.id();

  let {
    label,
    icon,
    onactivate,
    appearance = 'normal',
    disabled = false,
    checked,
    pending = false,
  }: {
    label: string;
    icon?: IconName;
    onactivate: Activation;
    appearance?: MenuItemAppearance;
    disabled?: boolean;
    checked?: boolean | null;
    pending?: boolean;
  } = $props();

  const unavailable = $derived(disabled || pending || checked === null);
</script>

<button
  type="button"
  class="flex w-full touch-pan-y flex-row flex-wrap items-center gap-2 px-4 py-3 text-left hover:bg-surface disabled:cursor-not-allowed disabled:hover:bg-transparent"
  class:text-muted={unavailable}
  class:text-danger={!unavailable && (appearance === 'destructive' || appearance === 'danger')}
  class:text-warning={!unavailable && appearance === 'warning'}
  disabled={unavailable}
  role={typeof checked === 'boolean' ? 'switch' : undefined}
  aria-checked={typeof checked === 'boolean' ? checked : undefined}
  aria-label={label}
  aria-describedby={checked === null ? statusId : undefined}
  aria-busy={pending || undefined}
  onpointerdown={(event) => activateFromPointer(event, onactivate, { scrollable: true })}
  onclick={(event) => activateFromKeyboardOrAssistiveClick(event, onactivate)}
>
  {#if icon || appearance === 'warning'}<Icon name={icon ?? 'triangle-alert'} class="mr-1 text-xl" />{/if}
  <span
    class="min-w-0 flex-1 wrap-anywhere whitespace-normal"
    style:flex-basis={typeof checked === 'boolean' ? '10ch' : undefined}>{label}</span
  >
  {#if checked === null}
    <span id={statusId} role="status" class="text-xs text-muted">Unknown</span>
  {/if}
  {#if typeof checked === 'boolean'}
    <ToggleIndicator {checked} />
  {/if}
</button>
