import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${name}: session Energy starts at zero and shows net consumption without Charge`,
    { timeout: 60_000 },
    async () => {
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
        const energyRequests: string[][] = [];
        await page.route('**/api/session-data', async (route) => {
          const response = await route.fetch();
          const value = await response.json();
          if (value.kind === 'samples' && value.values.wattHoursDrawnWh) {
            energyRequests.push(value.fields.map((field: { name: string }) => field.name));
            // Keep the mock's two samples inside the ordinary continuity threshold.
            value.elapsedMs = value.elapsedMs.map((ms: number) => ms / 60);
            value.values.wattHoursDrawnWh = value.elapsedMs.map((ms: number) => 800 + ms / 20);
            value.values.wattHoursChargedWh = value.elapsedMs.map((ms: number) => 100 + ms / 50);
          }
          await route.fulfill({ response, json: value });
        });
        await page.goto(`${endpoint.origin}/#/sessions/session-1`);
        const energy = page.getByRole('img', { name: 'Energy over elapsed time', exact: true });
        await energy.scrollIntoViewIfNeeded();
        await energy.locator('.telemetry-chart-trace').first().waitFor();
        assert.equal(await page.getByRole('img', { name: 'Charge over elapsed time', exact: true }).count(), 0);
        assert.ok(energyRequests.length > 0);
        assert.ok(energyRequests.every((fields) => fields.join(',') === 'wattHoursDrawnWh,wattHoursChargedWh'));
        const range = page.locator('[aria-label="Energy range"]');
        await range.getByText('50.0 Wh', { exact: true }).waitFor();
        assert.equal(await energy.locator('.telemetry-chart-trace').count(), 3);
        // All three curves share the same zero at the start despite nonzero controller totals.
        const starts = await energy
          .locator('.telemetry-chart-trace')
          .evaluateAll((paths) => paths.map((path) => path.getAttribute('d')!.split(' L ')[0]));
        assert.equal(new Set(starts).size, 1);
        await page.getByRole('button', { name: 'Hide Consumed line', exact: true }).tap();
        await range.getByText('30.0 Wh', { exact: true }).waitFor();
        await page.getByRole('button', { name: 'Hide Recovered line', exact: true }).tap();
        assert.equal(await energy.locator('.telemetry-chart-trace').count(), 1);
        const net = page.getByRole('button', { name: 'Hide Net consumed line', exact: true });
        await net.focus();
        await page.keyboard.press('Enter');
        await range.getByText('Max —', { exact: true }).waitFor();
        assert.equal(await energy.locator('.telemetry-chart-trace').count(), 0);
        await page.getByRole('button', { name: 'Show Net consumed line', exact: true }).tap();
        await range.getByText('30.0 Wh', { exact: true }).waitFor();
      } finally {
        await browser.close();
        await stopMockServer();
      }
    }
  );
}
