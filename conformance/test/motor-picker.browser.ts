import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');

for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: board motor selection saves a documented variant`, { timeout: 30_000 }, async () => {
    const port = await startMockServer(11002);
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
      const savedDeviceId = ready.activeConnection!.savedDeviceId!;
      const profile = await client.request('boardProfile.get', { savedDeviceId });
      await client.request('boardProfile.update', {
        savedDeviceId,
        expectedRevision: profile.revision,
        patch: { seriesCount: 18, parallelCount: 2 },
      });
      const page = await browser.newPage({ hasTouch: true });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((message) => server.send(message));
        server.onMessage((message) => socket.send(message));
      });
      await page.goto(`${endpoint.origin}/#/live`);
      await page.getByRole('button', { name: 'Board actions', exact: true }).click();
      await page.getByRole('button', { name: 'Configure board', exact: true }).click();
      const motor = page.getByRole('combobox', { name: 'Motor', exact: true });
      await motor.waitFor();
      assert.equal(await motor.locator('option:not([disabled])').count(), 15);
      await motor.selectOption('cannoncore_v3');
      const deadline = performance.now() + 5000;
      while ((await client.request('boardProfile.get', { savedDeviceId })).values.motor !== 'cannoncore_v3') {
        assert.ok(performance.now() < deadline, 'Motor choice was not saved');
        await new Promise((resolve) => setTimeout(resolve, 25));
      }
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
