import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: ambiguous controller guidance and activation`, { timeout: 60_000 }, async () => {
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
      await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const page = await browser.newPage({ hasTouch: true });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      const selections: number[] = [];
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (message.method === 'device.selectController') {
            selections.push(message.params.canId);
            return; // Hold the request to verify pending feedback and duplicate suppression.
          }
          server.send(raw);
        });
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state =
            message.event === 'state.changed' ? message.data : message.result?.activeConnection ? message.result : null;
          if (state?.activeConnection) {
            state.activeConnection.state = 'selecting_controller';
            state.activeConnection.controllerConnection = null;
            state.activeConnection.controllerCandidates = [1, 2].map((canId) => ({
              canId,
              hardwareName: 'Thor300',
              firmwareName: null,
              vescVersion: '6.06',
              vescSupported: true,
            }));
          }
          socket.send(JSON.stringify(message));
        });
      });
      for (const viewport of [
        { width: 390, height: 844 },
        { width: 1280, height: 800 },
      ]) {
        await page.setViewportSize(viewport);
        await page.goto(`${endpoint.origin}/#/live`);
        const panel = page.getByRole('dialog', { name: 'Choose controller', exact: true });
        await panel.waitFor();
        assert.deepEqual(await panel.boundingBox(), { x: 0, y: 0, ...viewport });
        assert.match(await panel.innerText(), /2 CAN addresses responded/);
        assert.doesNotMatch(
          await panel.innerText(),
          /Refloat compatibility is checked after|Matching names and versions/
        );
        assert.doesNotMatch(await panel.innerText(), /Unknown firmware|2 found|More than one VESC/);
        if (name === 'chromium' && process.env.FLOAT_VIEW_SCREENSHOTS) {
          await mkdir(process.env.FLOAT_VIEW_SCREENSHOTS, { recursive: true });
          await page.evaluate(() => {
            document.documentElement.dataset.theme = 'dark';
          });
          await page.screenshot({
            path: `${process.env.FLOAT_VIEW_SCREENSHOTS}/controller-selection-${viewport.width}.png`,
          });
        }
        const controller = panel.getByRole('button', { name: /CAN 1 Connect/ });
        const before = selections.length;
        await controller.click({ button: 'right' });
        assert.equal(selections.length, before);
        if (viewport.width === 390) await controller.tap();
        else {
          await controller.focus();
          await page.keyboard.press('Enter');
        }
        await panel.getByText('Connecting…', { exact: true }).waitFor();
        await controller.dispatchEvent('click', { detail: 1 });
        assert.deepEqual(selections.slice(before), [1]);
        assert.equal(await controller.isDisabled(), true);
      }
    } finally {
      await browser.close();
      await client.close();
      await stopMockServer();
    }
  });
}
