import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: compact selectors and long choices fit narrow touch screens`, { timeout: 30_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const browser = await engine.launch();
    try {
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      const scanned = await client.waitForState((state) => state.discoveredDevices.length > 0);
      await client.request('device.connect', {
        deviceId: scanned.discoveredDevices[0]!.deviceId,
        trackingMode: 'saved',
      });
      const ready = await client.waitForState((state) => state.activeConnection?.state === 'ready');
      const sessionId = ready.activeConnection!.activeSession!.sessionId;
      for (let index = 0; index < 3; index++) {
        await client.request('session.marker.add', { sessionId, operationId: randomUUID() });
        await new Promise((resolve) => setTimeout(resolve, 550));
      }
      const page = await browser.newPage({
        hasTouch: true,
        reducedMotion: 'reduce',
        viewport: { width: 390, height: 844 },
      });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.route('https://tile.openstreetmap.org/**', (route) => route.abort());
      await page.goto(`${endpoint.origin}/#/live`);
      await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
      await page.getByRole('button', { name: 'Configure Refloat', exact: true }).tap();
      const section = page.getByRole('combobox', { name: 'Section', exact: true });
      await section.waitFor();
      for (const width of [280, 320, 390]) {
        await page.setViewportSize({ width, height: 844 });
        assert.ok(Math.round((await section.boundingBox())!.height) >= 44);
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
      }
      await section.focus();
      await page.keyboard.press('Tab');
      await page.keyboard.press('Shift+Tab');
      assert.equal(await section.evaluate((el) => el === document.activeElement), true);
      assert.notEqual(await section.evaluate((el) => getComputedStyle(el).outlineStyle), 'none');
      const startup = await section.locator('option').filter({ hasText: 'Startup defaults' }).getAttribute('value');
      await section.selectOption(startup!);
      await page.getByRole('heading', { name: 'Lighting · Startup defaults', exact: true }).waitFor();
      await page.getByRole('button', { name: 'Done', exact: true }).tap();
      await client.request('device.disconnect', {});
      await page.goto(`${endpoint.origin}/#/sessions/${sessionId}`);
      const markers = page.getByRole('group', { name: 'Select ride marker', exact: true });
      await markers.waitFor();
      assert.equal(await markers.getByRole('button').count(), 3);
      for (const width of [280, 320, 390]) {
        await page.setViewportSize({ width, height: 844 });
        for (const button of await markers.getByRole('button').all()) {
          const size = await button.evaluate((el) => ({
            width: el.getBoundingClientRect().width,
            height: el.getBoundingClientRect().height,
            fits: el.scrollWidth <= el.clientWidth,
          }));
          assert.ok(size.width >= 44 && size.height >= 44);
          assert.ok(size.fits, `Long marker label overflows at ${width}px`);
        }
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
      }
      await markers.getByRole('button').last().tap();
      assert.equal(await markers.getByRole('button').last().getAttribute('aria-pressed'), 'true');
    } finally {
      await browser.close();
      await client.close();
      await stopMockServer();
    }
  });
}
