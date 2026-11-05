<script lang="ts">
  import SegmentedControl from '$lib/ui/SegmentedControl.svelte';
  let { label, checked, onchange }: { label: string; checked: boolean; onchange: (value: boolean) => unknown } =
    $props();
</script>

<div class="space-y-2">
  <p class="text-sm font-medium">{label}</p>
  <SegmentedControl
    {label}
    options={[
      { value: false, label: 'Off' },
      { value: true, label: 'On' },
    ]}
    value={checked}
    {onchange}
  />
</div>
