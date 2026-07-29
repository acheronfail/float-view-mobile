<script lang="ts">
	import { page } from '$app/state';
	import BoardEditor from '$lib/features/boards/BoardEditor.svelte';

	const savedDeviceId = $derived(page.params.savedDeviceId ?? '');
</script>

{#if savedDeviceId.length > 0 && savedDeviceId.length <= 512}
	<BoardEditor {savedDeviceId} />
{:else}
	<section class="rounded-2xl border border-outline bg-surface p-5">
		<a class="text-accent" href="#/boards">← Boards</a>
		<p role="alert" class="mt-3 text-warning">This board identifier is invalid.</p>
	</section>
{/if}
