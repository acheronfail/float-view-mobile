import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: diagnostic events load on expansion and follow chart zoom`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002);
    const endpoint = getMockBootstrap(port);
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({ hasTouch: true, reducedMotion: 'reduce' });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const requests: { range: { startElapsedMs: number; endElapsedMs: number }; cursor: string | null }[] = [];
      const errors: string[] = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.route('https://tile.openstreetmap.org/**', (route) => route.abort());
      await page.route('**/api/session-data', async (route) => {
        const request = route.request().postDataJSON();
        if (request.kind === 'events') requests.push(request);
        const response = await route.fetch(request.kind === 'events' ? { postData: { ...request, cursor: null } } : {});
        const value = await response.json();
        if (request.kind === 'events') {
          const more = !!request.cursor;
          value.page = {
            elapsedMs: [request.range.startElapsedMs + (more ? 1 : 0)],
            sequence: [more ? 2 : 1],
            sourceSampleSequence: [1],
            kinds: ['test'],
            values: [more ? 'second-page' : 'first-page'],
          };
          value.nextCursor = more ? null : 'test-cursor';
        }
        await route.fulfill({ response, json: value });
      });
      await page.goto(`${endpoint.origin}/#/sessions/session-1`);
      const toggle = page.getByRole('button', { name: 'State transitions', exact: true });
      const section = page.getByRole('region', { name: 'State transitions', exact: true });
      await toggle.waitFor();
      const chart = page.locator('svg[aria-label$="over elapsed time"]').first();
      await chart.waitFor();
      assert.equal(await toggle.getAttribute('aria-expanded'), 'false');
      assert.equal(requests.length, 0);
      await toggle.dispatchEvent('pointerdown', { button: 2, isPrimary: true, pointerId: 1 });
      await toggle.dispatchEvent('pointerup', { button: 2, isPrimary: true, pointerId: 1 });
      await toggle.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
      await toggle.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 2 });
      assert.equal(await toggle.getAttribute('aria-expanded'), 'false');
      await toggle.tap();
      await section.getByText(/test: first-page/).waitFor();
      assert.equal(requests.length, 1);
      assert.deepEqual(requests[0]!.range, { startElapsedMs: 0, endElapsedMs: 60000 });
      await section.getByRole('button', { name: 'Load more' }).tap();
      await section.getByText(/test: second-page/).waitFor();
      assert.equal(requests.length, 2);
      assert.equal(requests[1]!.cursor, 'test-cursor');
      // Zoom using the chart's real gesture path.
      await page.getByRole('button', { name: 'Chart mode: info. Switch to zoom mode' }).tap();
      await chart.scrollIntoViewIfNeeded();
      const box = (await chart.boundingBox())!;
      const pointer = {
        pointerId: 1,
        pointerType: 'mouse',
        isPrimary: true,
        button: 0,
        buttons: 1,
        clientY: box.y + 50,
      };
      await chart.dispatchEvent('pointerdown', { ...pointer, clientX: box.x + box.width * 0.25 });
      await chart.dispatchEvent('pointermove', { ...pointer, clientX: box.x + box.width * 0.75 });
      await chart.dispatchEvent('pointerup', { ...pointer, buttons: 0, clientX: box.x + box.width * 0.75 });
      await page.waitForFunction(() => {
        const text = document.querySelector('#session-events-content')?.textContent;
        return text?.includes('Current chart range:') && !text.includes('0–60000');
      });
      await section.getByText(/test: first-page/).waitFor();
      assert.equal(requests.length, 3);
      assert.ok(Math.abs(requests[2]!.range.startElapsedMs - 15000) < 100);
      assert.ok(Math.abs(requests[2]!.range.endElapsedMs - 45000) < 100);
      assert.equal(requests[2]!.cursor, null);
      assert.equal(await section.locator('li').count(), 1);
      await toggle.focus();
      await page.keyboard.press('Enter');
      assert.equal(await toggle.getAttribute('aria-expanded'), 'false');
      // Reset the chart while collapsed; only reopening should fetch the new range.
      await page
        .getByRole('button')
        .filter({ hasText: /^Reset$/ })
        .tap();
      await page.waitForTimeout(200);
      assert.equal(requests.length, 3);
      await toggle.focus();
      await page.keyboard.press('Space');
      await section.getByText(/test: first-page/).waitFor();
      assert.equal(requests.length, 4);
      assert.deepEqual(requests[3]!.range, requests[0]!.range);
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}
