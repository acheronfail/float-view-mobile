import tailwindcss from '@tailwindcss/vite';
import { icons as lucideIcons } from '@iconify-json/lucide';
import presetIcons from '@unocss/preset-icons';
import UnoCSS from 'unocss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, type Plugin, type ProxyOptions } from 'vite';
import { MOCK_BACKEND_PORT, startMockServer, stopMockServer } from './backend-mock';
import { iconClasses } from './src/lib/ui/icon';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { parseVersion } from '../.github/scripts/version';

let revision: string | null = null;
try {
  revision = execFileSync('git', ['describe', '--always', '--dirty', '--exclude=*', '--abbrev=40'], {
    cwd: import.meta.dirname,
    encoding: 'utf8',
  }).trim();
} catch {
  // Source archives can be built without Git metadata.
}
const version = process.env.FLOAT_VIEW_VERSION ?? null;
if (version !== null) parseVersion(version);

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
    rewriteWsOrigin: target !== MOCK_BACKEND_ORIGIN,
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

// Check the actual client chunks, not just package.json dependencies: Svelte and
// other build dependencies can contribute runtime code to the shipped HTML.
function checkBundledNotices(): Plugin {
  let ssr = false;
  return {
    name: 'float-view-bundled-notices',
    apply: 'build',
    configResolved(config) {
      ssr = Boolean(config.build.ssr);
    },
    generateBundle(_options, bundle) {
      if (ssr) return;
      const notices = readFileSync(new URL('../THIRD_PARTY_NOTICES.txt', import.meta.url), 'utf8');
      const packages = new Set<string>();
      for (const chunk of Object.values(bundle)) {
        if (chunk.type !== 'chunk') continue;
        for (const [id, module] of Object.entries(chunk.modules)) {
          if (!module.renderedLength || !id.includes('/node_modules/')) continue;
          const relative = id.split('/node_modules/').at(-1)!;
          const parts = relative.split('/');
          packages.add(parts.slice(0, relative.startsWith('@') ? 2 : 1).join('/'));
        }
      }
      for (const name of packages) {
        const { version } = JSON.parse(
          readFileSync(new URL(`./node_modules/${name}/package.json`, import.meta.url), 'utf8')
        );
        if (!notices.includes(`Frontend: ${name} ${version}\n`)) {
          this.error(`Missing reviewed runtime notices for bundled ${name} ${version}`);
        }
      }
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
    checkBundledNotices(),
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
