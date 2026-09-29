import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { test } from 'node:test';
import { chromium, webkit, type Page } from 'playwright';
import ts from 'typescript';

const template = await readFile(new URL('../../frontend/src/app.html', import.meta.url), 'utf8');
const builtHTML = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
const stylesheet = [...builtHTML.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((match) => match[1]).join('\n');
assert.ok(stylesheet.includes('--app-canvas'), 'Build the frontend before running startup rendering checks');
const theme = ts.transpileModule(
  await readFile(new URL('../../frontend/src/lib/app/theme.ts', import.meta.url), 'utf8'),
  { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }
).outputText;
const colors = { light: 'rgb(248, 250, 252)', dark: 'rgb(2, 6, 23)' };

async function appearance(page: Page) {
  return page.evaluate(() => ({
    background: getComputedStyle(document.documentElement).backgroundColor,
    scheme: getComputedStyle(document.documentElement).colorScheme,
    theme: document.documentElement.dataset.theme,
    meta: document.querySelector('meta[name="theme-color"]')?.getAttribute('content'),
  }));
}

for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: real early HTML canvas and authoritative theme reconciliation`, { timeout: 90_000 }, async (t) => {
    // No application stylesheet or modules arrive until the test explicitly loads them.
    const html = template.replace('%sveltekit.head%', '').replace('%sveltekit.body%', '<main>Startup probe</main>');
    const server = createServer((request, response) => {
      const path = request.url;
      response.setHeader(
        'Content-Type',
        path === '/theme.js' ? 'text/javascript' : path === '/app.css' ? 'text/css' : 'text/html'
      );
      response.end(path === '/theme.js' ? theme : path === '/app.css' ? stylesheet : html);
    });
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
    const address = server.address();
    assert.ok(address && typeof address !== 'string');
    t.after(() => new Promise<void>((resolve, reject) => server.close((error) => (error ? reject(error) : resolve()))));
    const browser = await engine.launch();
    try {
      for (const system of ['light', 'dark'] as const) {
        for (const hint of [null, 'system', 'light', 'dark', 'invalid', 'throw']) {
          await t.test(`${system} system, ${hint ?? 'missing'} hint`, async () => {
            const context = await browser.newContext({ colorScheme: system });
            try {
              await context.addInitScript((hint) => {
                if (hint === 'throw') {
                  Object.defineProperty(window, 'localStorage', {
                    get() {
                      throw new Error('Storage unavailable');
                    },
                  });
                } else if (hint !== null) {
                  localStorage.setItem('float-view-theme', hint);
                }
              }, hint);
              const page = await context.newPage();
              const errors: string[] = [];
              page.on('pageerror', (error) => errors.push(error.message));
              await page.goto(`http://127.0.0.1:${address.port}`);
              const initial = hint === 'light' || hint === 'dark' ? hint : system;
              assert.deepEqual(await appearance(page), {
                background: colors[initial],
                scheme: initial,
                theme: initial,
                meta: initial === 'dark' ? '#020617' : '#f8fafc',
              });
              // Apply the actual application CSS and controller only after checking early paint.
              await page.addStyleTag({ url: `http://127.0.0.1:${address.port}/app.css` });
              const authoritative = initial === 'dark' ? 'light' : 'dark';
              await page.evaluate(async (authoritative) => {
                const modulePath = '/theme.js';
                const { ThemeController, browserThemeEnvironment } = await import(modulePath);
                const controller = new ThemeController(browserThemeEnvironment());
                const stop = controller.start();
                Object.assign(window, { startupProbe: { controller, stop } });
                controller.reconcile(authoritative);
              }, authoritative);
              assert.equal((await appearance(page)).background, colors[authoritative]);
              if (hint !== 'throw') {
                assert.equal(await page.evaluate(() => localStorage.getItem('float-view-theme')), authoritative);
              }
              // Explicit preferences survive OS changes; system re-selection uses the new OS value.
              const changedSystem = system === 'dark' ? 'light' : 'dark';
              await page.emulateMedia({ colorScheme: changedSystem });
              assert.equal((await appearance(page)).background, colors[authoritative]);
              await page.evaluate(() => {
                const probe = (
                  window as unknown as {
                    startupProbe: { controller: { reconcile(value: string): void }; stop(): void };
                  }
                ).startupProbe;
                probe.controller.reconcile('system');
              });
              await page.waitForFunction(
                (expected) => document.documentElement.dataset.theme === expected,
                changedSystem
              );
              assert.equal((await appearance(page)).background, colors[changedSystem]);
              await page.evaluate(() => {
                (window as unknown as { startupProbe: { stop(): void } }).startupProbe.stop();
              });
              await page.emulateMedia({ colorScheme: system });
              assert.equal((await appearance(page)).background, colors[changedSystem]);
              assert.deepEqual(errors, []);
            } finally {
              await context.close();
            }
          });
        }
      }
    } finally {
      await browser.close();
    }
  });
}

for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: production frontend exposes backend startup failure`, { timeout: 30_000 }, async (t) => {
    const server = createServer((request, response) => {
      if (request.url === '/api/bootstrap') {
        response.writeHead(503, { 'Content-Type': 'application/json' });
        response.end('{}');
      } else {
        response.writeHead(200, { 'Content-Type': 'text/html' });
        response.end(builtHTML);
      }
    });
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
    t.after(() => new Promise<void>((resolve, reject) => server.close((error) => (error ? reject(error) : resolve()))));
    const address = server.address();
    assert.ok(address && typeof address !== 'string');
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({ colorScheme: 'dark' });
      await page.goto(`http://127.0.0.1:${address.port}`);
      await page.getByText('Bootstrap failed: HTTP 503', { exact: true }).waitFor({ state: 'visible' });
      assert.equal(await page.getByRole('link', { name: 'settings', exact: true }).isVisible(), true);
      assert.equal((await appearance(page)).background, colors.dark);
    } finally {
      await browser.close();
    }
  });
}
