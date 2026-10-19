import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import {
  startMockServer,
  stopMockServer,
  getMockBootstrap,
  simulateMockCanReplyConflict,
} from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: CAN warning survives live updates and clears on reconnect`, { timeout: 30000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const browser = await engine.launch();
    try {
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      const scan = await client.waitForState((s) => s.discoveredDevices.length > 0);
      const deviceId = scan.discoveredDevices[0]!.deviceId;
      await client.request('device.connect', { deviceId, trackingMode: 'saved' });
      await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const context = await browser.newContext({ viewport: { width: 390, height: 700 }, hasTouch: true });
      if (name === 'chromium') await context.grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await context.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      const page = await context.newPage();
      page.setDefaultTimeout(5000);
      await page.goto(`${endpoint.origin}/`);
      await page.getByRole('button', { name: 'Board actions', exact: true }).waitFor();
      const warning = page.getByRole('status', { name: 'Possible CAN ID conflict' });
      assert.equal(await warning.count(), 0);
      simulateMockCanReplyConflict();
      const warned = await client.waitForState((s) => !!s.activeConnection?.connectionWarning);
      await warning.waitFor();
      assert.match((await warning.textContent()) ?? '', /different CAN IDs/);
      await client.waitForState((s) => s.revision > warned.revision);
      assert.ok(await warning.isVisible());
      await page.getByRole('button', { name: 'Board actions', exact: true }).tap();
      assert.equal(await page.getByRole('alertdialog').count(), 0);
      await page.keyboard.press('Escape');
      await client.request('device.disconnect', {});
      await client.request('device.connect', { deviceId, trackingMode: 'saved' });
      await client.waitForState(
        (s) => s.activeConnection?.state === 'ready' && s.activeConnection.connectionWarning === null
      );
      await warning.waitFor({ state: 'detached' });
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
