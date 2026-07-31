<script lang="ts">
  import ControlPanel from '$lib/control/ControlPanel.svelte';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();
</script>

<ControlPanel initial={data.snapshot} token={data.token} />
