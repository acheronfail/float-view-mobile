<script lang="ts">
	import '../app.css';
	let { children } = $props();
</script>

<svelte:head>
	<meta name="theme-color" content="#0b1020" />
</svelte:head>

<main class="min-h-screen bg-slate-950 text-slate-100">
	{@render children?.()}
</main>
