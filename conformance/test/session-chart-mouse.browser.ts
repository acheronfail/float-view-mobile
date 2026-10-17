import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: mouse hover inspects, left drag zooms and right drag pans`, { timeout: 60_000 }, async () => {
    const endpoint = getMockBootstrap(await startMockServer(11002, { safetyDisclaimerAccepted: true }));
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({ viewport: { width: 1100, height: 760 } });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.route('https://tile.openstreetmap.org/**', (route) => route.abort());
      await page.route('**/api/session-data', async (route) => {
        const response = await route.fetch();
        const value = await response.json();
        if (value.kind === 'series') {
          for (const column of Object.values(value.columns)) {
            Object.assign(column as object, {
              elapsedMs: [0, 60000],
              sequence: [1, 2],
              generations: [0, 0],
              values: [1, 2],
            });
          }
        }
        await route.fulfill({ response, json: value });
      });
      const ranges: { startElapsedMs: number; endElapsedMs: number }[] = [];
      page.on('request', (request) => {
        if (!request.url().endsWith('/api/session-data')) return;
        const body = request.postDataJSON();
        if (body.kind === 'series') ranges.push(body.range);
      });
      await page.goto(`${endpoint.origin}/#/sessions/session-1`);
      await page.getByRole('button', { name: 'Show/hide the map', exact: true }).click();
      const chart = page.getByRole('img', { name: 'Current over elapsed time', exact: true });
      await chart.scrollIntoViewIfNeeded();
      await chart.locator('.telemetry-chart-trace').first().waitFor({ state: 'attached' });
      assert.equal(await page.getByRole('button', { name: /^Chart mode:/ }).count(), 0);
      const box = (await chart.boundingBox())!;
      const move = (ratio: number) => page.mouse.move(box.x + box.width * ratio, box.y + box.height / 2);
      await move(0.3);
      await chart.locator('..').getByRole('status').waitFor();
      // A click and a non-primary button drag must not change the viewport.
      await page.mouse.down();
      await page.mouse.up();
      await move(0.2);
      await page.mouse.down({ button: 'middle' });
      await move(0.6);
      await page.mouse.up({ button: 'middle' });
      const reset = page.getByRole('button', { name: 'Reset chart zoom', exact: true });
      assert.equal(await reset.count(), 0);

      const zoomed = page.waitForRequest((request) => {
        if (!request.url().endsWith('/api/session-data')) return false;
        const body = request.postDataJSON();
        return body.kind === 'series' && body.range.startElapsedMs > 0 && body.range.endElapsedMs < 60000;
      });
      await move(0.2);
      await page.mouse.down();
      await move(0.6);
      await page.mouse.up();
      const zoom = (await zoomed).postDataJSON().range;
      assert.ok(Math.abs(zoom.startElapsedMs - 12000) < 200);
      assert.ok(Math.abs(zoom.endElapsedMs - 36000) < 200);
      await reset.waitFor();
      const resetBox = (await reset.boundingBox())!;
      assert.ok(resetBox.x > 1000 && resetBox.y > 650 && resetBox.y + resetBox.height <= 760);
      for (const activation of [
        { button: 2, isPrimary: true },
        { button: 0, isPrimary: false },
      ]) {
        await reset.dispatchEvent('pointerdown', { ...activation, pointerType: 'mouse', pointerId: 42 });
        await reset.dispatchEvent('pointerup', { ...activation, pointerType: 'mouse', pointerId: 42 });
        assert.equal(await reset.count(), 1, 'secondary and non-primary activation keep the zoom');
      }
      await chart.locator('.telemetry-chart-trace').first().waitFor({ state: 'attached' });

      const panned = page.waitForRequest((request) => {
        if (!request.url().endsWith('/api/session-data')) return false;
        const body = request.postDataJSON();
        return body.kind === 'series' && body.range.startElapsedMs > zoom.startElapsedMs + 1000;
      });
      await move(0.6);
      await page.mouse.down({ button: 'right' });
      for (const ratio of [0.56, 0.52, 0.48, 0.44, 0.4]) {
        await move(ratio);
        // Let each preview update the scale before the next pointer event.
        await page.evaluate(() => new Promise<void>((resolve) => requestAnimationFrame(() => resolve())));
      }
      await page.mouse.up({ button: 'right' });
      const pan = (await panned).postDataJSON().range;
      assert.ok(Math.abs(pan.endElapsedMs - pan.startElapsedMs - (zoom.endElapsedMs - zoom.startElapsedMs)) < 2);
      const expectedDelta = (zoom.endElapsedMs - zoom.startElapsedMs) * 0.2;
      assert.ok(
        Math.abs(pan.startElapsedMs - zoom.startElapsedMs - expectedDelta) < 100,
        'pan follows the full cursor displacement'
      );
      await chart.locator('.telemetry-chart-trace').first().waitFor({ state: 'attached' });
      await move(0.5);
      await chart.locator('..').getByRole('status').waitFor();
      // Floating reset stays in reach even when a different chart is scrolled into view.
      await page.getByRole('img', { name: 'Voltage over elapsed time', exact: true }).scrollIntoViewIfNeeded();
      assert.deepEqual(await reset.boundingBox(), resetBox);
      await reset.focus();
      await page.keyboard.press('Enter');
      await reset.waitFor({ state: 'detached' });
      const voltage = page.getByRole('img', { name: 'Voltage over elapsed time', exact: true });
      await voltage.locator('.telemetry-chart-trace').first().waitFor({ state: 'attached' });
      await voltage.scrollIntoViewIfNeeded();
      await voltage.hover();
      const voltageBox = (await voltage.boundingBox())!;
      await page.mouse.move(voltageBox.x + voltageBox.width * 0.2, voltageBox.y + voltageBox.height / 2);
      await page.mouse.down();
      await page.mouse.move(voltageBox.x + voltageBox.width * 0.6, voltageBox.y + voltageBox.height / 2);
      await page.mouse.up();
      await reset.click();
      await reset.waitFor({ state: 'detached' });
      assert.ok(ranges.some((range) => range.startElapsedMs === 0 && range.endElapsedMs === 60000));
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}
