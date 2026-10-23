import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import {
  startMockServer,
  stopMockServer,
  getMockBootstrap,
  simulateConnectionFailure,
} from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
const compatibility = {
  reportedVersion: '1.3.1-beta',
  supportedVersions: [
    '1.0.0',
    '1.0.1',
    '1.1.0',
    '1.1.1',
    '1.1.2',
    '1.2.0',
    '1.2.1',
    '1.2.2',
    '1.2.3',
    '1.3.0',
    '1.3.1-beta1',
  ],
};

for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${name}: unsupported Refloat explains compatibility and supports explicit recovery`,
    { timeout: 60_000 },
    async () => {
      const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
      const endpoint = getMockBootstrap(port);
      const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
      const browser = await engine.launch();
      try {
        await client.request('bluetooth.scan', { timeoutMs: 100 });
        const scan = await client.waitForState((s) => s.discoveredDevices.length > 0);
        const deviceId = scan.discoveredDevices[0]!.deviceId;
        await client.request('device.connect', { deviceId, trackingMode: 'guest' });
        await client.waitForState((s) => s.activeConnection?.state === 'ready');
        simulateConnectionFailure('unsupported_refloat', 'Unsupported package', compatibility);
        const context = await browser.newContext({
          viewport: { width: 320, height: 568 },
          hasTouch: true,
          reducedMotion: 'reduce',
        });
        if (name === 'chromium') await context.grantPermissions(['local-network-access'], { origin: endpoint.origin });
        await context.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        let retries = 0;
        let exits = 0;
        let rejectNextExit = true;
        await context.routeWebSocket('**/api/ws?*', (socket) => {
          const server = socket.connectToServer();
          server.onMessage((raw) => socket.send(raw));
          socket.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            if (message.method === 'device.connect') {
              retries++;
              assert.equal(message.params.trackingMode, 'guest');
            }
            if (message.method === 'device.disconnect') {
              exits++;
              if (rejectNextExit) {
                rejectNextExit = false;
                socket.send(
                  JSON.stringify({ id: message.id, error: { code: 'unavailable', message: 'Disconnect unavailable' } })
                );
                return;
              }
            }
            server.send(raw);
          });
        });
        const page = await context.newPage();
        page.setDefaultTimeout(5_000);
        const errors: string[] = [];
        page.on('pageerror', (e) => errors.push(e.message));
        await page.goto(`${endpoint.origin}/#/live`);
        const dialog = page.getByRole('alertdialog', { name: 'Refloat version unsupported', exact: true });
        await dialog.waitFor();
        assert.match(await dialog.innerText(), /running Refloat 1\.3\.1-beta/);
        assert.equal(await page.getByText('Unsupported Refloat', { exact: true }).count(), 1);
        assert.equal(await page.getByText('Reconnecting', { exact: true }).count(), 0);
        assert.equal(await page.getByText('export diagnostic logs in Settings').count(), 0);
        const disclosure = dialog.getByRole('button', { name: 'View supported versions', exact: true });
        await disclosure.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
        assert.equal(await disclosure.getAttribute('aria-expanded'), 'false');
        await disclosure.tap();
        assert.equal(await disclosure.getAttribute('aria-expanded'), 'true');
        assert.match(await dialog.innerText(), /1\.0\.0, 1\.0\.1/);
        assert.match(await dialog.innerText(), /1\.3\.0/);
        assert.match(await dialog.innerText(), /1\.3\.1-beta1/);
        const back = dialog.getByRole('button', { name: 'Back to boards', exact: true });
        const bounds = await back.boundingBox();
        assert.ok(bounds && bounds.x >= 0 && bounds.x + bounds.width <= 320 && bounds.y + bounds.height <= 568);
        // Recreate the frontend: the backend's failed snapshot restores the explanation.
        await page.reload();
        await dialog.waitFor();
        const retry = dialog.getByRole('button', { name: 'Check again', exact: true });
        await retry.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 3 });
        assert.equal(retries, 0);
        await retry.tap();
        await client.waitForState((s) => s.activeConnection?.state === 'ready');
        simulateConnectionFailure('unsupported_refloat', 'Unsupported package', compatibility);
        await dialog.waitFor();
        assert.equal(retries, 1);
        // Retry acknowledgement may still be pending when its failure snapshot arrives.
        await back.click({ trial: true });
        await page.keyboard.press('Escape');
        await dialog.getByRole('alert').waitFor();
        assert.equal(exits, 1);
        assert.match(await dialog.innerText(), /Disconnect unavailable/);
        await back.focus();
        await page.keyboard.press('Enter');
        await client.waitForState((s) => !s.activeConnection);
        await dialog.waitFor({ state: 'hidden' });
        assert.equal(exits, 2);
        assert.ok(page.url().endsWith('#/'));
        assert.deepEqual(errors, []);
      } finally {
        await browser.close();
        client.close();
        await stopMockServer();
      }
    }
  );
}
