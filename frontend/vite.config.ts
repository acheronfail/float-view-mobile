import tailwindcss from '@tailwindcss/vite';
import { icons as lucideIcons } from '@iconify-json/lucide';
import presetIcons from '@unocss/preset-icons';
import UnoCSS from 'unocss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, type Plugin, type ProxyOptions } from 'vite';
import { MOCK_BACKEND_PORT, startMockServer, stopMockServer } from './backend-mock';
import { iconClasses } from './src/lib/ui/icon';
import { execFileSync } from 'node:child_process';

let revision: string | null = null;
try {
  revision = execFileSync('git', ['describe', '--always', '--dirty', '--exclude=*', '--abbrev=7'], {
    cwd: import.meta.dirname,
    encoding: 'utf8',
  }).trim();
} catch {
  // Source archives can be built without Git metadata.
}
const version = process.env.FLOAT_VIEW_VERSION ?? null;
if (version !== null && !/^v0\.1\.(0|[1-9][0-9]*)$/.test(version)) {
  throw new Error('FLOAT_VIEW_VERSION must be a v0.1.x release tag');
}

export const DEVELOPMENT_PORT = 11002;
export const MOCK_BACKEND_ORIGIN = `http://127.0.0.1:${MOCK_BACKEND_PORT}`;

export function backendProxyOrigin(target: string, incomingOrigin: string | undefined): string | undefined {
  if (!incomingOrigin) return undefined;
  return target === MOCK_BACKEND_ORIGIN ? incomingOrigin : target;
}

export function selectedBackendOrigin(environment: NodeJS.ProcessEnv = process.env): string {
  const value = environment.FLOAT_VIEW_DEV_BACKEND_ORIGIN ?? MOCK_BACKEND_ORIGIN;
  const url = new URL(value);
  if (url.protocol !== 'http:' || url.hostname !== '127.0.0.1' || url.pathname !== '/') {
    throw new Error('FLOAT_VIEW_DEV_BACKEND_ORIGIN must be an http://127.0.0.1:<port> origin');
  }
  return url.origin;
}

export function backendProxy(target: string): ProxyOptions {
  return {
    target,
    ws: true,
    changeOrigin: true,
    rewriteWsOrigin: true,
    timeout: 5_000,
    proxyTimeout: 5_000,
    configure(proxy) {
      // Native loopback hosts validate the application origin, not the Vite
      // development origin. HTTP proxying does not apply http-proxy's
      // WebSocket origin rewrite, so forward the target origin explicitly for
      // native backends. The mock backend intentionally authenticates the
      // browser development origin and must retain its incoming value.
      if (target !== MOCK_BACKEND_ORIGIN) {
        proxy.on('proxyReq', (request, incoming) => {
          const origin = backendProxyOrigin(target, incoming.headers.origin);
          if (origin) request.setHeader('Origin', origin);
        });
      }
      // http-proxy does not relay a non-101 response received by its WebSocket
      // client. Forward it explicitly so authentication failures stay bounded.
      proxy.on('proxyReqWs', (request, _incoming, socket) => {
        request.once('response', (response) => {
          const status = `HTTP/1.1 ${response.statusCode ?? 500} ${response.statusMessage ?? 'WebSocket rejected'}`;
          const headers: string[] = [];
          for (let index = 0; index < response.rawHeaders.length; index += 2) {
            headers.push(`${response.rawHeaders[index]}: ${response.rawHeaders[index + 1]}`);
          }
          socket.write(`${status}\r\n${headers.join('\r\n')}\r\n\r\n`);
          response.pipe(socket);
        });
      });
    },
  };
}

function mockBackend(target: string): Plugin {
  return {
    name: 'float-view-mock-backend',
    apply: 'serve',
    async configureServer(server) {
      if (process.env.VITEST || target !== MOCK_BACKEND_ORIGIN) return;
      await startMockServer(MOCK_BACKEND_PORT);
      server.httpServer?.once('close', () => void stopMockServer());
    },
  };
}

const backendOrigin = selectedBackendOrigin();

export default defineConfig({
  define: {
    __FLOAT_VIEW_BUILD_INFO__: JSON.stringify({ version, revision }),
  },
  plugins: [
    mockBackend(backendOrigin),
    tailwindcss(),
    UnoCSS({
      safelist: Object.values(iconClasses),
      presets: [
        presetIcons({
          mode: 'mask',
          collections: { lucide: () => lucideIcons },
          extraProperties: {
            display: 'inline-block',
            'vertical-align': 'middle',
          },
        }),
      ],
    }),
    sveltekit(),
  ],

  build: {
    rollupOptions: {
      output: {
        hashCharacters: 'hex',
      },
    },
  },

  server: {
    host: '127.0.0.1',
    port: DEVELOPMENT_PORT,
    strictPort: true,
    proxy: {
      '/api': backendProxy(backendOrigin),
    },
  },

  test: {
    expect: { requireAssertions: true },
    environment: 'node',
    include: ['src/**/*.{test,spec}.{js,ts}'],
  },
});
