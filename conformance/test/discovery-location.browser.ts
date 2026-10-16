import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { permissionInventory } from '../../frontend/backend-mock/permissions.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: discovery explains location and requests first-use access once`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({ hasTouch: true });
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const rows = permissionInventory('android31');
      const location = rows.find((row) => row.permission === 'location')!;
      let requests = 0;
      let scans = 0;
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (message.method === 'bluetooth.scan') scans++;
          if (message.method === 'permissions.recover') {
            assert.equal(message.params.permission, 'location');
            requests++;
            location.status = 'denied';
            location.action = 'openSettings';
            socket.send(JSON.stringify({ id: message.id, result: rows }));
            return;
          }
          server.send(raw);
        });
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (message.result?.permissions) message.result.permissions = rows;
          if (message.data?.permissions) message.data.permissions = rows;
          socket.send(JSON.stringify(message));
        });
      });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.goto(`${endpoint.origin}/#/`);
      const settings = page.getByRole('button', { name: 'Open settings', exact: true });
      await settings.waitFor();
      assert.match(await page.locator('main').innerText(), /Location access is needed to find boards/);
      assert.match(await page.locator('main').innerText(), /Ride route recording stays optional/);
      await page.waitForTimeout(1500);
      assert.equal(requests, 1);
      assert.equal(scans, 0);
      await settings.dispatchEvent('pointerdown', { button: 2, isPrimary: true, pointerId: 2 });
      await settings.dispatchEvent('pointerup', { button: 2, isPrimary: true, pointerId: 2 });
      assert.equal(requests, 1);
      await settings.tap();
      await page.waitForFunction(() => !document.querySelector<HTMLButtonElement>('main button')?.disabled);
      assert.equal(requests, 2);
      await settings.focus();
      await page.keyboard.press('Enter');
      await page.waitForFunction(() => !document.querySelector<HTMLButtonElement>('main button')?.disabled);
      assert.equal(requests, 3);
      location.status = 'granted';
      location.accuracy = 'precise';
      location.servicesEnabled = false;
      await page.reload();
      await page.getByRole('heading', { name: 'Location is off', exact: true }).waitFor();
      assert.equal(scans, 0);
      location.servicesEnabled = true;
      await page.reload();
      await page.getByText('Discovered boards', { exact: true }).waitFor();
      assert.ok(scans > 0);
      assert.equal(requests, 3);
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}
