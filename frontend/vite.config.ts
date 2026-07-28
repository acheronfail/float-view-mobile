import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, type Plugin } from 'vite';
import { startMockServer, stopMockServer } from './backend-mock';

function mockBackend(): Plugin {
	return {
		name: 'float-view-mock-backend',
		apply: 'serve',
		async configureServer(server) {
			if (process.env.VITEST) return;
			await startMockServer(3001);
			server.httpServer?.once('close', () => void stopMockServer());
		}
	};
}

export default defineConfig({
	plugins: [mockBackend(), tailwindcss(), sveltekit()],

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
