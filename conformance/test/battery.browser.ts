import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: battery configuration autosave and shared live displays`, { timeout: 60_000 }, async () => {
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
        trackingMode: 'saved',
      });
      const ready = await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const id = ready.activeConnection!.savedDeviceId!;
      const profile = await client.request('boardProfile.get', { savedDeviceId: id });
      await client.request('boardProfile.update', {
        savedDeviceId: id,
        expectedRevision: profile.revision,
        patch: { seriesCount: 18, parallelCount: 2 },
      });
      const page = await browser.newPage({ hasTouch: true, reducedMotion: 'reduce' });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const errors: string[] = [];
      const methods: string[] = [];
      page.on('pageerror', (e) => errors.push(e.message));
      page.on('websocket', (socket) =>
        socket.on('framesent', (event) => {
          try {
            const message = JSON.parse(String(event.payload));
            if (message.method) methods.push(message.method);
          } catch {
            /* handshake */
          }
        })
      );
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.goto(`${endpoint.origin}/#/live`);
      const gauge = page.getByRole('progressbar', { name: 'Estimated battery level', exact: true });
      await gauge.waitFor();
      assert.equal(await gauge.getAttribute('aria-valuetext'), 'Unavailable');
      await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
      await page.getByRole('button', { name: 'Configure board', exact: true }).tap();
      const selector = page.getByLabel('Battery cell profile', { exact: true });
      const minimum = page.getByLabel('Display empty voltage per cell', { exact: true });
      await selector.waitFor({ timeout: 5000 }).catch(async (error) => {
        console.error(await page.locator('body').innerText(), errors);
        throw error;
      });
      const before = methods.filter((m) => m === 'boardProfile.update').length;
      await selector.dispatchEvent('pointerdown', { button: 2, isPrimary: true });
      await selector.dispatchEvent('pointerup', { button: 2, isPrimary: true });
      assert.equal(methods.filter((m) => m === 'boardProfile.update').length, before);
      await selector.selectOption('molicel-p45b');
      await selector.dispatchEvent('click', { detail: 1 });
      await client.waitForState((s) => s.activeConnection?.batteryEstimate.profileId === 'molicel-p45b');
      assert.equal(methods.filter((m) => m === 'boardProfile.update').length, before + 1);
      await minimum.fill('4.2');
      await minimum.press('Tab');
      await page.getByRole('alert').filter({ hasText: 'Enter a voltage' }).waitFor();
      assert.equal(methods.filter((m) => m === 'boardProfile.update').length, before + 1);
      // Force the editor's revision stale, then save a valid voltage via keyboard blur.
      const current = await client.request('boardProfile.get', { savedDeviceId: id });
      await client.request('boardProfile.update', {
        savedDeviceId: id,
        expectedRevision: current.revision,
        patch: { motor: 'test' },
      });
      await minimum.fill('2.7');
      await minimum.press('Tab');
      const deadline = performance.now() + 5000;
      let saved = await client.request('boardProfile.get', { savedDeviceId: id });
      while (saved.values.batteryMinimumCellVoltageV !== 2.7 && performance.now() < deadline) {
        await new Promise((resolve) => setTimeout(resolve, 25));
        saved = await client.request('boardProfile.get', { savedDeviceId: id });
      }
      assert.equal(saved.values.batteryMinimumCellVoltageV, 2.7);
      assert.ok(saved.revision >= current.revision + 2);
      await page
        .getByRole('dialog', { name: 'Board configuration' })
        .getByRole('button', { name: 'Done', exact: true })
        .click();
      const percent = await gauge.getAttribute('aria-valuenow');
      assert.ok(percent && Number(percent) >= 0);
      await page.getByRole('button', { name: 'Board actions', exact: true }).click();
      await page.getByRole('button', { name: 'Detailed View', exact: true }).focus();
      await page.keyboard.press('Enter');
      await page.getByText('Estimated battery level:', { exact: true }).waitFor();
      const estimatedRow = page.getByText('Estimated battery level:', { exact: true }).locator('..');
      assert.ok((await estimatedRow.innerText()).includes(`${percent}%`));
      await page.getByText('Controller-reported battery level:', { exact: true }).waitFor();
      assert.equal(
        methods.some((m) => /poll|telemetry/i.test(m)),
        false
      );
      await page.reload();
      await gauge.waitFor();
      assert.equal(await gauge.getAttribute('aria-valuenow'), percent);
      await page.getByRole('button', { name: 'Board actions', exact: true }).click();
      await page.getByRole('button', { name: 'Configure board', exact: true }).click();
      assert.equal(await selector.inputValue(), 'molicel-p45b');
      assert.equal(await minimum.inputValue(), '2.7');
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
