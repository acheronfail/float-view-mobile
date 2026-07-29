<script lang="ts">
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { backendState } from '$lib/backend/state.svelte';
	import BottomTabBar from './BottomTabBar.svelte';
	import { bottomTabForRoute } from './navigation';
	import { browserThemeEnvironment, ThemeController } from './theme';

	let { children }: { children: Snippet } = $props();
	let themeController: ThemeController | undefined;
	const activeBottomTab = $derived(bottomTabForRoute(page.route.id));

	onMount(() => {
		themeController = new ThemeController(browserThemeEnvironment());
		const stopTheme = themeController.start();
		void backendState.connect();
		return () => {
			stopTheme();
			themeController = undefined;
			backendState.close();
		};
	});

	$effect(() => {
		const preference = backendState.settings?.themePreference;
		if (!preference) return;
		themeController?.reconcile(preference);
	});

	$effect(() => {
		if (
			backendState.status === 'connected' &&
			backendState.snapshot?.activeConnection &&
			page.route.id === '/'
		) {
			void goto('#/live', { replaceState: true });
		}
	});
</script>

<svelte:head>
	<meta name="theme-color" content="#020617" />
</svelte:head>

<main class="min-h-screen bg-canvas text-foreground">
	<div
		class="mx-auto flex min-h-screen max-w-3xl flex-col px-4 pt-6"
		class:pb-28={activeBottomTab}
		class:pb-6={!activeBottomTab}
	>
		<header class="mb-5">
			<h1 class="text-3xl font-semibold">Float View</h1>
			<p class="text-sm text-muted">Durable VESC sessions</p>
		</header>

		<div class="flex-1">
			{@render children()}
		</div>
	</div>
	<BottomTabBar routeId={page.route.id} />
</main>
