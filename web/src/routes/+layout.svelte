<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<meta name="theme-color" content="#0b1020" />
</svelte:head>

<main class="min-h-screen bg-slate-950 text-slate-100">
	{@render children?.()}
</main>
