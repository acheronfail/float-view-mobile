<script lang="ts">
	let { title, items }: { title: string; items: [string, string][] } = $props();
</script>

<section aria-label={title}>
	<h3 class="mb-1 text-xs font-semibold tracking-[0.16em] text-muted uppercase">{title}</h3>
	<dl
		class="grid grid-cols-2 overflow-hidden rounded-xl border border-outline sm:grid-cols-2 lg:grid-cols-3"
	>
		{#each items as item (item[0])}
			<div class="min-w-0 border-b border-outline bg-surface p-3 last:border-b-0 sm:border-r">
				<dt class="text-[0.625rem] tracking-wide text-muted uppercase">{item[0]}</dt>
				<dd class="mt-1 font-mono text-base font-medium break-words">{item[1]}</dd>
			</div>
		{/each}
	</dl>
</section>
