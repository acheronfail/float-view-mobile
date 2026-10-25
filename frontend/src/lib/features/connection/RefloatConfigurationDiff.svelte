<script lang="ts">
  import type { RefloatConfigurationField, RefloatConfigurationValue } from '$lib/backend/contract';
  import { formatRefloatValue, refloatConfigurationFieldLabel } from './refloat-configuration.svelte';

  let {
    changes,
    fields,
    beforeLabel,
    afterLabel,
    label = 'Configuration differences',
    emptyMessage = 'No differences.',
  }: {
    changes: readonly {
      name: string;
      label: string;
      before: RefloatConfigurationValue | undefined;
      after: RefloatConfigurationValue | undefined;
    }[];
    fields: readonly RefloatConfigurationField[];
    beforeLabel: string;
    afterLabel: string;
    label?: string;
    emptyMessage?: string;
  } = $props();

  const fieldsByName = $derived(new Map(fields.map((field) => [field.name, field])));
</script>

{#if changes.length}
  <p class="mb-2 text-sm font-medium">
    {changes.length}
    {changes.length === 1 ? 'setting will change' : 'settings will change'}
  </p>
  <p class="mb-2 text-xs text-muted">{beforeLabel} → {afterLabel}</p>
  <ul class="space-y-1 text-sm [overflow-wrap:anywhere]" aria-label={label}>
    {#each changes as item (item.name)}
      {@const field = fieldsByName.get(item.name)}
      <li>
        <strong>{field ? refloatConfigurationFieldLabel(field) : item.label}:</strong>
        {item.before ? formatRefloatValue(item.before, field) : 'Not available'} → {item.after
          ? formatRefloatValue(item.after, field)
          : 'Not available'}
      </li>
    {/each}
  </ul>
{:else}
  <p class="text-sm text-muted">{emptyMessage}</p>
{/if}
