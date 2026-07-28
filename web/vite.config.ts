import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],

	build: {
		rollupOptions: {
			output: {
				hashCharacters: 'hex'
			}
		}
	},

	server: {
		proxy: {
			'/api': {
				target: 'http://127.0.0.1:3001',
				ws: true,
				changeOrigin: true
			}
		}
	},

	test: {
		expect: { requireAssertions: true },
		environment: 'node',
		include: ['src/**/*.{test,spec}.{js,ts}']
	}
});
