<script lang="ts" generics="T extends string | number | boolean">
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from './activation';

  let {
    label,
    options,
    value,
    disabled = false,
    describedby,
    id,
    onchange,
  }: {
    label: string;
    options: { value: T; label: string; disabled?: boolean; group?: string }[];
    value: T;
    disabled?: boolean;
    describedby?: string;
    id?: string;
    onchange: (value: T) => void;
  } = $props();

  const choices = $derived(options.filter((option) => !option.disabled));
  const groups = $derived([...new Set(options.map((option) => option.group))]);
  function select(next: T) {
    if (!disabled && next !== value) onchange(next);
  }
</script>

{#if choices.length > 0 && choices.length <= 3}
  <div
    {id}
    role="group"
    aria-label={label}
    aria-describedby={describedby}
    class="flex w-full gap-1 rounded-xl border border-outline bg-surface p-1"
  >
    {#each choices as option (option.value)}
      <button
        type="button"
        disabled={disabled || option.disabled}
        aria-pressed={value === option.value}
        class="min-h-11 min-w-0 flex-1 touch-manipulation rounded-lg px-2 py-2 text-sm font-medium break-words text-muted transition-colors select-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent disabled:opacity-50 aria-pressed:bg-accent aria-pressed:text-on-accent aria-pressed:shadow-sm"
        onpointerdown={(event) => activateFromPointer(event, () => select(option.value))}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => select(option.value))}
        >{option.label}</button
      >
    {/each}
  </div>
{:else}
  <select
    {id}
    {disabled}
    aria-label={label}
    aria-describedby={describedby}
    class="min-h-11 w-full touch-manipulation rounded-md border border-outline bg-elevated px-3 py-2 text-sm text-foreground"
    value={String(value)}
    onchange={(event) => {
      const option = options.find((option) => String(option.value) === event.currentTarget.value);
      if (!disabled && option && !option.disabled) onchange(option.value);
    }}
  >
    {#each groups as group (group)}
      {#if group}
        <optgroup label={group}>
          {#each options.filter((option) => option.group === group) as option (option.value)}
            <option value={String(option.value)} disabled={option.disabled}>{option.label}</option>
          {/each}
        </optgroup>
      {:else}
        {#each options.filter((option) => !option.group) as option (option.value)}
          <option value={String(option.value)} disabled={option.disabled}>{option.label}</option>
        {/each}
      {/if}
    {/each}
  </select>
{/if}
