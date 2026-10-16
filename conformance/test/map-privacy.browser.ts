import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
const tile = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=',
  'base64'
);

for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: maps send only tile coordinates and retain visible attribution`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true });
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      page.setDefaultTimeout(8000);
      const requests: { url: string; headers: Record<string, string>; method: string }[] = [];
      const externalHosts = new Set<string>();
      page.on('request', (request) => {
        const url = new URL(request.url());
        if (url.hostname !== '127.0.0.1') externalHosts.add(url.hostname);
      });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      // Never send synthetic route data to the public tile service from automated tests.
      // Routing disables browser caches; native cache behavior is a separate audit.
      await page.route('https://tile.openstreetmap.org/**', async (route) => {
        const request = route.request();
        requests.push({ url: request.url(), headers: await request.allHeaders(), method: request.method() });
        await route.fulfill({ contentType: 'image/png', body: tile });
      });
      await page.goto(`${endpoint.origin}/#/settings`);
      await page.getByRole('heading', { name: 'Settings', exact: true }).waitFor();
      assert.equal(requests.length, 0, 'ordinary Settings does not request maps');
      await page.goto(`${endpoint.origin}/#/sessions/session-1`);
      const map = page.getByRole('img', { name: 'Recorded session route', exact: true });
      await map.waitFor();
      await page.waitForFunction(() => document.querySelectorAll('.leaflet-tile-loaded').length > 0);
      assert.ok(requests.length > 0, 'the route requests visible tiles');
      for (const request of requests) {
        const url = new URL(request.url);
        assert.equal(url.origin, 'https://tile.openstreetmap.org');
        assert.equal(url.search, '');
        assert.equal(url.hash, '');
        const match = /^\/(\d+)\/(\d+)\/(\d+)\.png$/.exec(url.pathname);
        assert.ok(match, 'only standard tile paths leave the device');
        assert.ok(Number(match[1]) <= 19);
        assert.equal(request.method, 'GET');
        assert.equal(request.headers.authorization, undefined);
        assert.equal(request.headers.cookie, undefined);
        if (request.headers.referer) assert.equal(request.headers.referer, `${endpoint.origin}/`);
      }
      assert.deepEqual([...externalHosts], ['tile.openstreetmap.org']);
      const attribution = map.locator('.leaflet-control-attribution');
      assert.match(await attribution.innerText(), /© OpenStreetMap contributors/);
      assert.equal(
        await attribution.getByRole('link', { name: 'OpenStreetMap', exact: true }).getAttribute('href'),
        'https://www.openstreetmap.org/copyright'
      );
      for (const width of [320, 390, 768]) {
        await page.setViewportSize({ width, height: 844 });
        const bounds = await map.boundingBox();
        const credit = await attribution.boundingBox();
        assert.ok(bounds && credit);
        assert.ok(credit.x >= bounds.x - 1 && credit.x + credit.width <= bounds.x + bounds.width + 1);
        assert.ok(credit.y >= bounds.y && credit.y + credit.height <= bounds.y + bounds.height + 1);
      }
      await page.getByRole('button', { name: 'Show/hide the map', exact: true }).click();
      await map.waitFor({ state: 'hidden' });
      await page.waitForLoadState('networkidle');
      const afterHide = requests.length;
      await page.evaluate(
        () => new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())))
      );
      assert.equal(requests.length, afterHide, 'a hidden map does not request more tiles');
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}
