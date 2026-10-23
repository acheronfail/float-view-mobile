import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(
    `${name}: a saved board restores its selected view without a process checkpoint`,
    { timeout: 60000 },
    async () => {
      const browser = await engine.launch();
      const port = await startMockServer(0, { safetyDisclaimerAccepted: true });
      const endpoint = getMockBootstrap(port);
      const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
      try {
        await client.request('bluetooth.requestPermission', {});
        await client.request('bluetooth.scan', { timeoutMs: 100 });
        const scanned = await client.waitForState((s) => s.discoveredDevices.length > 0);
        const firstDevice = scanned.discoveredDevices[0]!.deviceId;
        async function connect(deviceId: string) {
          await client.request('device.connect', { deviceId, trackingMode: 'saved' });
          const state = await client.waitForState(
            (s) => s.activeConnection?.deviceId === deviceId && s.activeConnection.state === 'ready'
          );
          return state.activeConnection!.savedDeviceId!;
        }
        const first = await connect(firstDevice);
        const page = await browser.newPage({
          viewport: { width: 390, height: 844 },
          hasTouch: true,
          reducedMotion: 'reduce',
        });
        page.setDefaultTimeout(7000);
        if (name === 'chromium')
          await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
        await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
        await page.route(`${endpoint.origin}/api/bootstrap`, (route) =>
          route.fulfill({ json: { webSocketUrl: endpoint.webSocketUrl, platform: 'web' } })
        );
        let writes = 0;
        let failSave = false;
        const errors: string[] = [];
        page.on('pageerror', (e) => errors.push(e.message));
        await page.routeWebSocket('**/api/ws?*', (socket) => {
          const server = socket.connectToServer();
          socket.onMessage((raw) => {
            const message = JSON.parse(String(raw));
            if (message.method === 'boardProfile.update') {
              writes++;
              if (failSave) {
                failSave = false;
                socket.send(
                  JSON.stringify({ id: message.id, error: { code: 'unavailable', message: 'Storage unavailable' } })
                );
                return;
              }
            }
            server.send(raw);
          });
          server.onMessage((raw) => socket.send(raw));
        });
        async function select(label: string) {
          await page.getByRole('button', { name: 'Board actions', exact: true }).click();
          await page.getByRole('button', { name: label, exact: true }).click();
        }
        async function selected(label: string) {
          await page.getByRole('button', { name: 'Board actions', exact: true }).click();
          assert.equal(
            await page.getByRole('button', { name: label, exact: true }).getAttribute('aria-current'),
            'page'
          );
          await page.keyboard.press('Escape');
        }
        async function coldReload() {
          await page.goto('about:blank');
          await client.request('uiState.set', { checkpoint: null });
          await page.goto(`${endpoint.origin}/#/live`);
        }
        await page.goto(`${endpoint.origin}/#/live`);
        await selected('Live View');
        assert.equal(writes, 0, 'default restoration does not write');
        failSave = true;
        await select('Custom View');
        await page.getByRole('alert').filter({ hasText: 'Could not save this board' }).waitFor();
        await selected('Live View');
        await select('Custom View');
        const custom = page.getByRole('region', { name: 'Custom dashboard', exact: true });
        await custom.waitFor();
        assert.equal(
          (await client.request('boardProfile.get', { savedDeviceId: first })).values.lastLiveView,
          'custom'
        );
        await custom
          .getByRole('button', { name: /^Arrange / })
          .first()
          .focus();
        await page.keyboard.press('Enter');
        await custom.getByRole('button', { name: 'Save dashboard', exact: true }).click();
        await custom.getByText('Press and hold any widget to arrange your dashboard.', { exact: true }).waitFor();
        const beforeRestore = writes;
        await coldReload();
        await custom.waitFor();
        assert.equal(writes, beforeRestore);
        await client.request('device.disconnect', {});
        await connect(firstDevice);
        await coldReload();
        await custom.waitFor();
        await select('Detailed View');
        await selected('Detailed View');
        await coldReload();
        await selected('Detailed View');
        assert.equal(
          (await client.request('boardProfile.get', { savedDeviceId: first })).values.lastLiveView,
          'detailed'
        );
        await select('Live View');
        await selected('Live View');
        await coldReload();
        await selected('Live View');
        assert.equal(
          (await client.request('boardProfile.get', { savedDeviceId: first })).values.lastLiveView,
          'normal'
        );
        assert.deepEqual(errors, []);
      } finally {
        client.close();
        await browser.close();
        await stopMockServer();
      }
    }
  );
}
