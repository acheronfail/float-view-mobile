import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';
import type { RefloatControls } from '../../frontend/src/lib/backend/contract.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: confirmed controls work during schema download`, { timeout: 60_000 }, async () => {
    const browser = await engine.launch();
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
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
      await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const active = (await client.request('state.get', {})).activeConnection!;
      const profile = await client.request('boardProfile.get', { savedDeviceId: active.savedDeviceId! });
      await client.request('boardProfile.update', {
        savedDeviceId: profile.savedDeviceId,
        expectedRevision: profile.revision,
        patch: {
          liveWidgetSlots: ['leds', 'headlights', 'empty', 'odometerM', 'distanceM'],
        },
      });
      const page = await browser.newPage({
        viewport: { width: 390, height: 844 },
        hasTouch: true,
        reducedMotion: 'reduce',
      });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const errors: string[] = [];
      page.on('pageerror', (e) => errors.push(e.message));
      const controls: RefloatControls = {
        status: 'loading',
        writable: true,
        packageEnabled: true,
        flywheelEnabled: false,
        lightingAvailable: true,
        ledsEnabled: true,
        headlightsEnabled: false,
        fatalErrorClearAvailable: true,
        imuTimeoutAvailable: false,
        fatalErrorLatched: null,
        fatalErrorActive: null,
        fatalErrorClearable: null,
        pendingAction: null,
      };
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => server.send(raw));
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state =
            message.event === 'state.changed' ? message.data : message.result?.activeConnection ? message.result : null;
          if (state?.activeConnection) {
            state.activeConnection.refloatControls = { ...controls };
            if (state.activeConnection.refloatConfiguration) {
              state.activeConnection.refloatConfiguration.availability = 'loading';
            }
            Object.assign(state.activeConnection.latestSample, {
              rideState: 'idle',
              packageState: 'ready',
              speedKph: 0,
            });
          }
          socket.send(JSON.stringify(message));
        });
      });
      await page.goto(`${endpoint.origin}/#/live`);
      const menu = page.getByRole('button', { name: 'Board actions', exact: true });
      await menu.tap();
      const actionOverlay = page.getByRole('dialog', { name: 'Board actions', exact: true });
      await actionOverlay.waitFor();
      const disablePackage = actionOverlay.getByRole('button', { name: 'Disable Refloat…', exact: true });
      assert.equal(await disablePackage.isEnabled(), true);
      assert.equal(
        await actionOverlay.getByRole('button', { name: 'Enable flywheel…', exact: true }).isEnabled(),
        true
      );
      assert.equal(await actionOverlay.getByRole('switch', { name: 'LEDs', exact: true }).isEnabled(), true);
      await disablePackage.tap();
      await page.getByRole('alertdialog', { name: 'Disable Refloat package?', exact: true }).waitFor();
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
