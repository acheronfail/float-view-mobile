import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';
import type { CustomDashboard } from '../../frontend/src/lib/backend/contract.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: custom marker button capture, editing and feedback`, { timeout: 60_000 }, async () => {
    const browser = await engine.launch();
    const port = await startMockServer(0, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    try {
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      const scanned = await client.waitForState((s) => s.discoveredDevices.length > 0);
      await client.request('device.connect', {
        deviceId: scanned.discoveredDevices[0]!.deviceId,
        trackingMode: 'saved',
      });
      const ready = await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const sessionId = ready.activeConnection!.activeSession!.sessionId;
      const savedDeviceId = ready.activeConnection!.savedDeviceId!;
      const profile = await client.request('boardProfile.get', { savedDeviceId });
      const dashboard: CustomDashboard = {
        version: 1,
        widgets: [
          { id: 'speed', kind: 'visual', visual: 'speedGauge', size: { columns: 3, rows: 3 } },
          { id: 'marker', kind: 'action', action: 'rideMarker', size: { columns: 1, rows: 1 } },
          { id: 'battery', kind: 'visual', visual: 'battery', size: { columns: 1, rows: 2 } },
          { id: 'temp', kind: 'visual', visual: 'temperatures', size: { columns: 2, rows: 2 } },
          { id: 'footpads', kind: 'visual', visual: 'footpads', size: { columns: 2, rows: 2 } },
        ],
      };
      await client.request('boardProfile.update', {
        savedDeviceId,
        expectedRevision: profile.revision,
        patch: { customDashboard: dashboard, lastLiveView: 'custom' },
      });
      const page = await browser.newPage({
        viewport: { width: 390, height: 844 },
        hasTouch: true,
        reducedMotion: 'reduce',
        colorScheme: name === 'chromium' ? 'dark' : 'light',
      });
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const errors: string[] = [];
      page.on('pageerror', (error) => errors.push(error.message));
      let fail = false;
      let hold: (() => void) | undefined;
      let delay = false;
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (message.method === 'session.marker.add' && fail) {
            socket.send(
              JSON.stringify({ id: message.id, error: { code: 'invalid_request', message: 'Marker rejected' } })
            );
          } else if (message.method === 'session.marker.add' && delay) {
            hold = () => server.send(raw);
          } else server.send(raw);
        });
        server.onMessage((raw) => socket.send(raw));
      });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.route(`${endpoint.origin}/api/bootstrap`, (route) =>
        route.fulfill({ json: { webSocketUrl: endpoint.webSocketUrl, platform: 'web' } })
      );
      page.setDefaultTimeout(7000);
      await page.goto(`${endpoint.origin}/#/live`);
      const button = page
        .locator('[data-dashboard-widget="marker"]')
        .getByRole('button', { name: 'Save ride marker', exact: true });
      await button.waitFor();
      const output = new URL(`../generated/marker-widget/${name}/`, import.meta.url);
      await mkdir(output, { recursive: true });
      await page.screenshot({ path: new URL('phone-ready.png', output).pathname });
      await button.tap();
      await page.getByRole('status').filter({ hasText: 'Marker saved' }).waitFor();
      assert.equal((await client.request('session.get', { sessionId })).markerCount, 1);
      await page.screenshot({ path: new URL('phone-saved.png', output).pathname });
      await page.waitForTimeout(550);
      await button.focus();
      await page.keyboard.press('Enter');
      await page.waitForTimeout(100);
      assert.equal((await client.request('session.get', { sessionId })).markerCount, 2);
      await page.waitForTimeout(550);
      await button.click({ button: 'right' });
      const box = (await button.boundingBox())!;
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.mouse.move(box.x + box.width / 2, box.y + box.height + 30);
      await page.mouse.up();
      assert.equal((await client.request('session.get', { sessionId })).markerCount, 2);
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await button.dispatchEvent('pointercancel', { pointerId: 1, isPrimary: true });
      await page.mouse.up();
      assert.equal((await client.request('session.get', { sessionId })).markerCount, 2);
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.waitForTimeout(700);
      await page.mouse.up();
      await page.getByRole('toolbar', { name: 'Edit dashboard' }).waitFor();
      assert.equal((await client.request('session.get', { sessionId })).markerCount, 2);
      await page.getByRole('button', { name: /Configure Save ride marker/ }).click();
      await page.getByLabel('Content', { exact: true }).selectOption('action:rideMarker');
      assert.equal((await client.request('session.get', { sessionId })).markerCount, 2);
      await page.getByRole('button', { name: 'Cancel', exact: true }).click();
      await page.getByRole('button', { name: 'Cancel', exact: true }).click();
      await button.waitFor();
      fail = true;
      await button.tap();
      await page.getByRole('status').filter({ hasText: 'Could not save' }).waitFor();
      assert.equal((await client.request('session.get', { sessionId })).markerCount, 2);
      fail = false;
      await page.setViewportSize({ width: 1280, height: 800 });
      await page.waitForTimeout(2100);
      await page.screenshot({ path: new URL('desktop-ready.png', output).pathname });
      delay = true;
      await button.click();
      await page.getByRole('status').filter({ hasText: 'Saving' }).waitFor();
      assert.equal(await button.isDisabled(), true);
      await client.request('device.disconnect', {});
      hold?.();
      await page.waitForTimeout(200);
      assert.equal(await page.getByRole('status').filter({ hasText: 'Marker saved' }).count(), 0);
      await page.reload();
      assert.deepEqual((await client.request('boardProfile.get', { savedDeviceId })).values.customDashboard, dashboard);
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
