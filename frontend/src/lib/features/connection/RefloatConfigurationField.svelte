<script lang="ts">
  import type { RefloatConfigurationField, RefloatConfigurationValue } from '$lib/backend/contract';

  let {
    field,
    value,
    disabled = false,
    error = null,
    onchange,
  }: {
    field: RefloatConfigurationField;
    value: RefloatConfigurationValue | undefined;
    disabled?: boolean;
    error?: string | null;
    onchange: (value: RefloatConfigurationValue) => void;
  } = $props();

  const persistentLighting = $derived(field.name === 'leds.on' || field.name === 'leds.headlights_on');
  const label = $derived(persistentLighting ? `${field.label} (persistent startup default)` : field.label);
  const helpId = $derived(`refloat-${field.name.replaceAll(/[^a-zA-Z0-9_-]/g, '-')}-help`);
</script>

<label class="block rounded-lg border border-outline bg-elevated p-3 text-sm">
  <span class="font-medium">{label}</span>
  {#if field.type === 'boolean'}
    <span class="mt-2 flex items-center gap-2">
      <input
        type="checkbox"
        class="h-5 w-5 touch-manipulation"
        checked={value?.type === 'boolean' && value.value}
        {disabled}
        aria-describedby={helpId}
        onchange={(event) => onchange({ type: 'boolean', value: event.currentTarget.checked })}
      />
      <span>{value?.type === 'boolean' && value.value ? 'On' : 'Off'}</span>
    </span>
  {:else if field.type === 'enum'}
    <select
      class="mt-2 w-full rounded-md border border-outline bg-surface px-3 py-2"
      {disabled}
      aria-describedby={helpId}
      value={value?.type === 'integer' ? value.value : 0}
      onchange={(event) => onchange({ type: 'integer', value: Number(event.currentTarget.value) })}
    >
      {#each field.choices as choice, index (index)}
        <option value={index}>{choice}</option>
      {/each}
    </select>
  {:else if field.type === 'string'}
    <input
      type="text"
      maxlength="4096"
      class="mt-2 w-full rounded-md border border-outline bg-surface px-3 py-2"
      {disabled}
      aria-describedby={helpId}
      value={value?.type === 'string' ? value.value : ''}
      oninput={(event) => onchange({ type: 'string', value: event.currentTarget.value })}
    />
  {:else}
    <div class="mt-2 flex items-center gap-2">
      <input
        type="number"
        class="min-w-0 flex-1 rounded-md border border-outline bg-surface px-3 py-2"
        min={field.minimum ?? undefined}
        max={field.maximum ?? undefined}
        step={field.step ?? (field.type === 'double' ? 'any' : 1)}
        {disabled}
        aria-describedby={helpId}
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
  <span id={helpId} class="mt-1 block text-xs text-muted">
    {field.description || field.name}
  </span>
  {#if error}<span role="alert" class="mt-1 block text-xs text-warning">{error}</span>{/if}
</label>
