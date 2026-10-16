import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: battery configuration autosave and shared live displays`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
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
      let projection: 'normal' | 'missing' | 'reconnecting' | 'other-board' = 'normal';
      let rejectSave = false;
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (rejectSave && message.method === 'boardProfile.update') {
            socket.send(
              JSON.stringify({ id: message.id, error: { code: 'invalid_request', message: 'Test save failure' } })
            );
          } else server.send(raw);
        });
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state =
            message.event === 'state.changed' ? message.data : message.result?.activeConnection ? message.result : null;
          if (state?.activeConnection) {
            if (projection === 'missing' && state.activeConnection.latestSample)
              state.activeConnection.latestSample.batteryLevel = null;
            if (projection === 'reconnecting') state.activeConnection.state = 'reconnecting';
            if (projection === 'other-board') state.activeConnection.savedDeviceId = 'other-board';
          }
          socket.send(JSON.stringify(message));
        });
      });
      await page.goto(`${endpoint.origin}/#/live`);
      const gauge = page.getByRole('progressbar', { name: 'Estimated battery level', exact: true });
      const reportedGauge = page.getByRole('progressbar', { name: 'Controller-reported battery level', exact: true });
      await reportedGauge.waitFor();
      assert.notEqual(await reportedGauge.getAttribute('aria-valuetext'), 'Unavailable');
      const warning = page.getByRole('button', { name: 'Battery setup required', exact: true });
      await warning.click({ button: 'right' });
      assert.equal(await page.getByRole('dialog').count(), 0);
      await warning.evaluate((el) =>
        el.addEventListener(
          'pointerdown',
          (event) => {
            el.setAttribute('data-test-pointer', String((event as PointerEvent).pointerId));
          },
          { once: true }
        )
      );
      await warning.hover();
      await page.mouse.down();
      const pointerId = Number(await warning.getAttribute('data-test-pointer'));
      await warning.dispatchEvent('pointercancel', { pointerId });
      await page.mouse.up();
      assert.equal(await page.getByRole('dialog').count(), 0);
      projection = 'missing';
      await page.waitForFunction(
        () =>
          document
            .querySelector('[role="progressbar"][aria-label="Controller-reported battery level"]')
            ?.getAttribute('aria-valuetext') === 'Unavailable'
      );
      assert.equal(await warning.count(), 1);
      projection = 'normal';
      await warning.tap();
      const batteryDialog = page.getByRole('dialog', { name: 'Battery configuration', exact: true });
      await batteryDialog.waitFor();
      assert.equal(await page.getByRole('dialog').count(), 1);
      assert.equal(await batteryDialog.getByLabel('Display name', { exact: true }).count(), 0);
      assert.equal(await batteryDialog.getByLabel('Motor', { exact: true }).count(), 0);
      const series = page.getByLabel('Battery series cells (S)', { exact: true });
      await series.fill('19');
      projection = 'reconnecting';
      await page.waitForTimeout(1100);
      assert.equal(await series.inputValue(), '19');
      assert.equal(await batteryDialog.count(), 1);
      await series.fill('18');
      projection = 'normal';
      await page.keyboard.press('Escape');
      await batteryDialog.waitFor({ state: 'hidden' });
      assert.equal(await warning.evaluate((el) => el === document.activeElement), true);
      await page.getByRole('button', { name: 'Board actions', exact: true }).click();
      await page.getByRole('button', { name: 'Detailed View', exact: true }).click();
      await page.getByRole('dialog', { name: 'Board actions', exact: true }).waitFor({ state: 'hidden' });
      await warning.focus();
      await page.keyboard.press('Enter');
      await batteryDialog.waitFor();
      const selector = page.getByLabel('Battery cell profile', { exact: true });
      const minimum = page.getByLabel('Display empty voltage per cell', { exact: true });
      await selector.waitFor({ timeout: 5000 }).catch(async (error) => {
        console.error(await page.locator('body').innerText(), errors);
        throw error;
      });
      rejectSave = true;
      await minimum.fill('2.9');
      await minimum.press('Tab');
      await batteryDialog.getByRole('alert').filter({ hasText: 'Test save failure' }).waitFor();
      await batteryDialog.getByRole('button', { name: 'Done', exact: true }).click();
      assert.equal(await batteryDialog.count(), 1);
      assert.notEqual(
        (await client.request('boardProfile.get', { savedDeviceId: id })).values.batteryMinimumCellVoltageV,
        2.9
      );
      await batteryDialog.getByRole('button', { name: 'Close without saving failed changes', exact: true }).click();
      await batteryDialog.waitFor({ state: 'hidden' });
      rejectSave = false;
      await warning.tap();
      await batteryDialog.waitFor();
      await minimum.fill('3.0');
      await minimum.press('Shift+Tab');
      await batteryDialog.getByRole('alert').filter({ hasText: 'Test save failure' }).waitFor({ state: 'hidden' });
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
        .getByRole('dialog', { name: 'Battery configuration' })
        .getByRole('button', { name: 'Done', exact: true })
        .click();
      assert.equal(await warning.count(), 0);
      await page.waitForFunction(() => document.activeElement?.getAttribute('aria-label') === 'Board actions');
      const configured = await client.waitForState((s) => s.activeConnection?.batteryEstimate.level != null);
      const percent = String(Math.round(configured.activeConnection!.batteryEstimate.level! * 100));
      await page.getByText('Estimated battery level:', { exact: true }).waitFor();
      const estimatedRow = page.getByText('Estimated battery level:', { exact: true }).locator('..');
      assert.ok((await estimatedRow.innerText()).includes(`${percent}%`));
      await page.getByText('Controller-reported battery level:', { exact: true }).waitFor();
      assert.equal(
        methods.some((m) => /poll|telemetry/i.test(m)),
        false
      );
      await page.reload();
      await page.getByRole('region', { name: 'Detailed board telemetry', exact: true }).waitFor();
      await page.getByText('Estimated battery level:', { exact: true }).waitFor();
      await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
      await page.getByRole('button', { name: 'Normal View', exact: true }).tap();
      await gauge.waitFor();
      assert.equal(await gauge.getAttribute('aria-valuenow'), percent);
      await page.getByRole('button', { name: 'Board actions', exact: true }).click();
      await page.getByRole('button', { name: 'Configure board', exact: true }).click();
      assert.equal(await selector.inputValue(), 'molicel-p45b');
      assert.equal(await minimum.inputValue(), '2.7');
      projection = 'other-board';
      await page.getByRole('dialog').waitFor({ state: 'hidden' });
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
