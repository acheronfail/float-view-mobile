import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: fault banner, contextual clear and conditional danger menu`, { timeout: 90_000 }, async () => {
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
        trackingMode: 'guest',
      });
      await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const page = await browser.newPage({
        hasTouch: true,
        reducedMotion: 'reduce',
        viewport: { width: 390, height: 844 },
      });
      page.setDefaultTimeout(10000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const errors: string[] = [];
      page.on('pageerror', (e) => errors.push(e.message));
      let phase: 'unknown' | 'active' | 'history' = 'unknown';
      let clearRequests = 0;
      let cleared = false;
      let projectedControls: Record<string, unknown> = {};
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => {
          const request = JSON.parse(String(raw));
          if (request.method === 'refloat.fatalError.clear') {
            clearRequests++;
            assert.equal(phase, 'unknown');
            cleared = true;
            socket.send(
              JSON.stringify({
                id: request.id,
                result: { ...projectedControls, fatalErrorLatched: false, fatalErrorClearable: false },
              })
            );
          } else server.send(raw);
        });
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state =
            message.event === 'state.changed' ? message.data : message.result?.activeConnection ? message.result : null;
          const active = state?.activeConnection;
          if (active?.latestSample) {
            active.latestSample.rideState = 'idle';
            active.latestSample.fatalErrorLatched = !cleared;
            Object.assign(active.refloatControls, {
              fatalErrorLatched: !cleared,
              fatalErrorActive: false,
              fatalErrorClearable: !cleared,
            });
            if (phase === 'active') {
              active.latestSample.firmwareFault = 'overVoltage';
              active.refloatControls.fatalErrorActive = true;
              active.refloatControls.fatalErrorClearable = false;
            }
            if (phase === 'history')
              active.latestSample.faultHistory = [
                { code: 1, reason: 'overVoltage', elapsedMs: 1000, observedElapsedMs: 2000 },
              ];
          }
          if (active?.refloatControls) projectedControls = { ...active.refloatControls };
          socket.send(JSON.stringify(message));
        });
      });
      await page.goto(`${endpoint.origin}/#/live`);
      const banner = page.getByRole('button', { name: /Open fault history:/ });
      await banner.waitFor();
      await banner.click({ button: 'right' });
      const dialog = page.getByRole('dialog', { name: 'Fault history', exact: true });
      assert.equal(await dialog.count(), 0);
      await banner.tap();
      await dialog.getByText(/check the fault log in VESC Tool/).waitFor();
      await dialog.getByText('No fault details recorded for this ride.', { exact: true }).waitFor();
      const clear = dialog.getByRole('button', { name: 'Clear fatal error', exact: true });
      assert.equal(await clear.isEnabled(), true);
      for (const width of [320, 390]) {
        await page.setViewportSize({ width, height: 844 });
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
      }
      await mkdir(new URL('../generated/', import.meta.url), { recursive: true });
      await page.screenshot({ path: new URL(`../generated/fault-actions-${name}.png`, import.meta.url).pathname });

      phase = 'active';
      await dialog.getByText(/It must end before the latch can be cleared/).waitFor();
      assert.equal(await clear.isDisabled(), true);
      assert.equal(clearRequests, 0);
      phase = 'unknown';
      await page.waitForFunction(
        () =>
          !document.querySelector<HTMLButtonElement>(
            '[role="dialog"] button[aria-describedby="fault-clear-unavailable"]'
          )
      );
      await clear.focus();
      await page.keyboard.press('Enter');
      await clear.waitFor({ state: 'hidden' });
      assert.equal(clearRequests, 1);
      await dialog.getByRole('button', { name: 'Done', exact: true }).tap();
      await dialog.waitFor({ state: 'hidden' });
      await banner.waitFor({ state: 'hidden' });
      const menu = page.getByRole('button', { name: 'Board actions', exact: true });
      await menu.tap();
      assert.equal(await page.getByRole('button', { name: 'Fault history…', exact: true }).count(), 0);
      assert.equal(await page.getByRole('button', { name: 'Clear fatal error', exact: true }).count(), 0);
      await page.keyboard.press('Escape');
      phase = 'history';
      await menu.tap();
      const history = page.getByRole('button', { name: 'Fault history…', exact: true });
      await history.waitFor();
      assert.equal(await history.evaluate((el) => el.classList.contains('text-danger')), true);
      await history.focus();
      await page.keyboard.press('Enter');
      await dialog.getByText('Overvoltage', { exact: true }).waitFor();
      assert.equal(await dialog.getByRole('button', { name: 'Clear fatal error', exact: true }).count(), 0);
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
