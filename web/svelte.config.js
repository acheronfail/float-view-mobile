import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://svelte.dev/docs/kit/integrations
	// for more information about preprocessors
	preprocess: vitePreprocess(),

	kit: {
		// Appears that Android doesn't copy assets which start with `_`, and sveltekit's
		// default is `_app`.
		appDir: 'app',
		paths: {
			// base: '/path'
		},
		adapter: adapter({
			pages: 'build',
			assets: 'build',
			fallback: undefined,
			precompress: false,
			strict: true
		}),
		prerender: {
			crawl: true,
			entries: ['*']
		}
	}
};

export default config;
