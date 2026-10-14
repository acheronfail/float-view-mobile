import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';
const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: live charts open, follow telemetry, show gaps and dismiss`, { timeout: 60000 }, async () => {
    const port = await startMockServer(11002);
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const browser = await engine.launch();
    try {
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      const scanned = await client.waitForState((s) => s.discoveredDevices.length > 0);
      await client.request('device.connect', {
        deviceId: scanned.discoveredDevices[0]!.deviceId,
        trackingMode: 'guest',
      });
      await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const page = await browser.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const errors: string[] = [];
      page.on('pageerror', (e) => errors.push(e.message));
      let frozen = false;
      let lastSample: unknown;
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state =
            message.event === 'state.changed' ? message.data : message.result?.activeConnection ? message.result : null;
          if (state?.activeConnection?.latestSample) {
            if (frozen) state.activeConnection.latestSample = lastSample;
            else {
              // Vary the deterministic mock readings to exercise scrolling geometry.
              const sample = state.activeConnection.latestSample;
              const phase = sample.sequence / 8;
              Object.assign(sample, {
                speedKph: 22 + Math.sin(phase) * 5,
                dutyCycle: 0.42 + Math.sin(phase) * 0.12,
                motorCurrentA: 18 + Math.sin(phase * 1.3) * 14,
                batteryCurrentA: 8 + Math.sin(phase * 1.3) * 7,
                packVoltageV: 58 - Math.sin(phase * 1.3) * 1.2,
                controllerTempC: 38 + Math.sin(phase / 3) * 2,
                motorTempC: 45 + Math.sin(phase / 3) * 3,
                pitchDeg: Math.sin(phase) * 3,
                rollDeg: Math.cos(phase) * 5,
                setpointDeg: Math.sin(phase) * 1.5,
              });
              lastSample = sample;
            }
          }
          socket.send(JSON.stringify(message));
        });
      });
      await page.goto(`${endpoint.origin}/#/live`);
      const menu = page.getByRole('button', { name: 'Board actions', exact: true });
      await menu.tap();
      const option = page.getByRole('button', { name: 'Live charts…', exact: true });
      await option.click({ button: 'right' });
      await option.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
      await option.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 2 });
      assert.equal(await menu.getAttribute('aria-expanded'), 'true');
      await option.tap();
      const charts = page.getByRole('dialog', { name: 'Live charts', exact: true });
      await charts.waitFor();
      assert.equal(await charts.getByRole('img').count(), 6);
      assert.equal(await charts.getByRole('button').count(), 1);
      const load = charts.getByRole('region', { name: 'Load', exact: true });
      assert.equal(await load.locator('path').count(), 3);
      assert.deepEqual(
        await load.locator('path').evaluateAll((paths) => paths.map((path) => path.getAttribute('stroke'))),
        ['var(--app-duty)', 'var(--app-danger)', 'var(--app-success)']
      );
      await page.waitForFunction(() =>
        Array.from(document.querySelectorAll('section[aria-label="Load"] path')).every((path) => path.getAttribute('d'))
      );
      assert.equal(await load.getByText('%', { exact: true }).count(), 3);
      const path = charts.locator('path').first();
      await page.waitForTimeout(1500);
      const before = await path.getAttribute('d');
      await page.waitForTimeout(500);
      assert.notEqual(await path.getAttribute('d'), before);
      const box = (await charts.boundingBox())!;
      assert.equal(box.width, 390);
      assert.equal(box.height, 844);
      const assertFullWidth = async (width: number) => {
        for (const chart of await charts.getByRole('img').all()) {
          const bounds = (await chart.boundingBox())!;
          assert.equal(bounds.x, 0);
          assert.equal(bounds.width, width);
        }
      };
      await assertFullWidth(390);
      await page.setViewportSize({ width: 1280, height: 1000 });
      await assertFullWidth(1280);
      await page.setViewportSize({ width: 390, height: 844 });
      frozen = true;
      await charts.getByRole('status').filter({ hasText: 'Waiting for live data' }).waitFor();
      assert.equal(
        await charts.getByRole('region', { name: 'Speed', exact: true }).getByText('—', { exact: true }).count(),
        1
      );
      frozen = false;
      await charts
        .getByRole('status')
        .filter({ hasText: /^Live$/ })
        .waitFor();
      // A longer capture shows actual rolling mock telemetry in screenshots.
      const output = process.env.FLOAT_VIEW_SCREENSHOTS;
      if (output && name === 'chromium') {
        await mkdir(output, { recursive: true });
        await page.waitForTimeout(12000);
        for (const theme of ['light', 'dark'] as const) {
          await client.request('settings.update', {
            expectedRevision: client.currentState()!.settings.revision,
            patch: { themePreference: theme },
          });
          await page.waitForFunction((value) => document.documentElement.dataset.theme === value, theme);
          await page.screenshot({ path: `${output}/live-charts-${theme}-mobile.png` });
          await page.setViewportSize({ width: 1280, height: 1000 });
          await page.screenshot({ path: `${output}/live-charts-${theme}-desktop.png` });
          await page.setViewportSize({ width: 390, height: 844 });
        }
      }
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
      await page.keyboard.press('Escape');
      await charts.waitFor({ state: 'hidden' });
      assert.equal(await menu.evaluate((el) => el === document.activeElement), true);
      await menu.focus();
      await page.keyboard.press('Enter');
      await option.focus();
      await page.keyboard.press('Enter');
      await charts.waitFor();
      const done = charts.getByRole('button', { name: 'Done', exact: true });
      await done.tap();
      await charts.waitFor({ state: 'hidden' });
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
