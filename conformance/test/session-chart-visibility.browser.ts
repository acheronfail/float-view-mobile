import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: session axes follow visible lines and preserve fixed ranges`, { timeout: 60_000 }, async () => {
    const endpoint = getMockBootstrap(await startMockServer(11002, { safetyDisclaimerAccepted: true }));
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({
        viewport: { width: 390, height: 844 },
        hasTouch: true,
        reducedMotion: 'reduce',
      });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.route('https://tile.openstreetmap.org/**', (route) => route.abort());
      await page.route('**/api/session-data', async (route) => {
        const response = await route.fetch();
        const value = await response.json();
        if (value.kind === 'series') {
          for (const [field, column] of Object.entries(value.columns)) {
            Object.assign(column as object, {
              elapsedMs: [0, 1000, 2000],
              sequence: [1, 2, 3],
              generations: [0, 0, 0],
              values:
                field === 'packVoltageV' ? [72, 74, 76] : field === 'chargingVoltageV' ? [0, 0, 0] : [0.2, 0.3, 0.4],
            });
          }
        }
        await route.fulfill({ response, json: value });
      });
      await page.goto(`${endpoint.origin}/#/sessions/session-1`);
      const voltage = page.getByRole('img', { name: 'Voltage over elapsed time', exact: true });
      await voltage.scrollIntoViewIfNeeded();
      await voltage.locator('.telemetry-chart-trace').nth(1).waitFor({ state: 'attached' });
      const frame = voltage.locator('..').locator('..');
      const ticks = frame.locator(':scope > div').first().locator('span');
      const originalTicks = await ticks.allTextContents();
      assert.equal(originalTicks.at(-1), '-3.80', 'enabled zero readings contribute to the axis');
      const packPath = voltage.locator('.telemetry-chart-trace').first();
      const originalPath = await packPath.getAttribute('d');
      await page.getByRole('button', { name: 'Hide Charging voltage line', exact: true }).tap();
      await ticks.getByText('71.80', { exact: true }).waitFor();
      assert.notDeepEqual(await ticks.allTextContents(), originalTicks);
      assert.notEqual(await packPath.getAttribute('d'), originalPath);
      assert.equal(await voltage.locator('.telemetry-chart-trace').count(), 1);
      await page.getByRole('button', { name: 'Show Charging voltage line', exact: true }).tap();
      await voltage.locator('.telemetry-chart-trace').nth(1).waitFor({ state: 'attached' });
      assert.deepEqual(await ticks.allTextContents(), originalTicks);
      assert.equal(await packPath.getAttribute('d'), originalPath);

      await page.getByRole('button', { name: 'Hide Charging voltage line', exact: true }).tap();
      await page.getByRole('button', { name: 'Hide Pack voltage line', exact: true }).focus();
      await page.keyboard.press('Enter');
      await frame.locator('..').getByText('All lines hidden.', { exact: true }).waitFor();
      assert.equal(await ticks.count(), 0);
      assert.equal(await voltage.locator('.telemetry-chart-trace').count(), 0);
      await page.getByRole('button', { name: 'Show Pack voltage line', exact: true }).tap();
      await ticks.getByText('71.80', { exact: true }).waitFor();

      const load = page.getByRole('img', { name: 'Load over elapsed time', exact: true });
      await load.scrollIntoViewIfNeeded();
      await load.locator('.telemetry-chart-trace').first().waitFor({ state: 'attached' });
      const loadTicks = load.locator('..').locator('..').locator(':scope > div').first().locator('span');
      const fixedTicks = await loadTicks.allTextContents();
      assert.equal(fixedTicks[0], '100%');
      assert.equal(fixedTicks.at(-1), '0%');
      await page.getByRole('button', { name: 'Hide Motor load line', exact: true }).tap();
      await page.getByRole('button', { name: 'Show Motor load line', exact: true }).waitFor();
      assert.deepEqual(await loadTicks.allTextContents(), fixedTicks);
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}
