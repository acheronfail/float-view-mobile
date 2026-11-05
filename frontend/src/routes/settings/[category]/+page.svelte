<script lang="ts">
  import { page } from '$app/state';
  import SettingsCategoryView from '$lib/features/settings/SettingsCategoryView.svelte';
</script>

<SettingsCategoryView category={page.params.category!} />
