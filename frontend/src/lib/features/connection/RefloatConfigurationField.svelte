<script lang="ts">
  import type { RefloatConfigurationField, RefloatConfigurationValue } from '$lib/backend/contract';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import { refloatConfigurationFieldLabel } from './refloat-configuration.svelte';

  let {
    field,
    value,
    disabled = false,
    error = null,
    onchange,
    onhelp,
  }: {
    field: RefloatConfigurationField;
    value: RefloatConfigurationValue | undefined;
    disabled?: boolean;
    error?: string | null;
    onchange: (value: RefloatConfigurationValue) => void;
    onhelp: (field: RefloatConfigurationField, trigger: HTMLButtonElement) => void;
  } = $props();

  const label = $derived(refloatConfigurationFieldLabel(field));
  const fieldId = $derived(`refloat-${field.name.replaceAll(/[^a-zA-Z0-9_-]/g, '-')}`);
  const errorId = $derived(`${fieldId}-error`);
  let helpButton = $state<HTMLButtonElement>();
</script>

<div class="block rounded-lg border border-outline bg-elevated p-3 text-sm">
  <span class="flex items-center justify-between gap-2">
    <label for={fieldId} class="font-medium">{label}</label>
    <button
      bind:this={helpButton}
      type="button"
      class="min-h-8 min-w-8 shrink-0 touch-manipulation rounded-md border border-outline px-1 text-xs"
      aria-label={`Show help for ${label}`}
      onpointerdown={(event) => activateFromPointer(event, () => helpButton && onhelp(field, helpButton))}
      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => helpButton && onhelp(field, helpButton))}
      >(?)</button
    >
  </span>
  {#if field.type === 'boolean'}
    <span class="mt-2 flex items-center gap-2">
      <input
        id={fieldId}
        type="checkbox"
        class="h-5 w-5 touch-manipulation"
        checked={value?.type === 'boolean' && value.value}
        {disabled}
        aria-describedby={error ? errorId : undefined}
        onchange={(event) => onchange({ type: 'boolean', value: event.currentTarget.checked })}
      />
      <span>{value?.type === 'boolean' && value.value ? 'On' : 'Off'}</span>
    </span>
  {:else if field.type === 'enum'}
    <select
      id={fieldId}
      class="mt-2 w-full rounded-md border border-outline bg-surface px-3 py-2"
      {disabled}
      aria-describedby={error ? errorId : undefined}
      value={value?.type === 'integer' ? value.value : 0}
      onchange={(event) => onchange({ type: 'integer', value: Number(event.currentTarget.value) })}
    >
      {#each field.choices as choice, index (index)}
        <option value={index}>{choice}</option>
      {/each}
    </select>
  {:else if field.type === 'string'}
    <input
      id={fieldId}
      type="text"
      maxlength="4096"
      class="mt-2 w-full rounded-md border border-outline bg-surface px-3 py-2"
      {disabled}
      aria-describedby={error ? errorId : undefined}
      value={value?.type === 'string' ? value.value : ''}
      oninput={(event) => onchange({ type: 'string', value: event.currentTarget.value })}
    />
  {:else}
    <div class="mt-2 flex items-center gap-2">
      <input
        id={fieldId}
        type="number"
        class="min-w-0 flex-1 rounded-md border border-outline bg-surface px-3 py-2"
        min={field.minimum ?? undefined}
        max={field.maximum ?? undefined}
        step={field.step ?? (field.type === 'double' ? 'any' : 1)}
        {disabled}
        aria-describedby={error ? errorId : undefined}
        value={value?.type === 'double' || value?.type === 'integer' ? value.value : ''}
        oninput={(event) => {
          const number = event.currentTarget.valueAsNumber;
          if (!Number.isFinite(number)) return;
          onchange(field.type === 'double' ? { type: 'double', value: number } : { type: 'integer', value: number });
        }}
      />
      {#if field.suffix}<span class="text-muted">{field.suffix}</span>{/if}
    </div>
  {/if}
  {#if error}<span id={errorId} role="alert" class="mt-1 block text-xs text-warning">{error}</span>{/if}
</div>
