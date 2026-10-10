import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';
import type { RefloatControls } from '../../frontend/src/lib/backend/contract.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: live menu switches, confirmations, pending and unknown state`, { timeout: 60_000 }, async () => {
    const browser = await engine.launch();
    const port = await startMockServer(11002);
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
        status: 'ready',
        writable: true,
        packageEnabled: true,
        flywheelEnabled: false,
        lightingAvailable: true,
        ledsEnabled: true,
        headlightsEnabled: false,
        fatalErrorClearAvailable: false,
        fatalErrorLatched: false,
        fatalErrorActive: false,
        fatalErrorClearable: false,
        pendingAction: null,
      };
      const requests: { control: string; enabled: boolean }[] = [];
      let complete: ((fail?: boolean) => void) | undefined;
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (message.method === 'refloat.control.set') {
            requests.push(message.params);
            controls.pendingAction = message.params.control;
            complete = (fail = false) => {
              controls.pendingAction = null;
              if (!fail) {
                if (message.params.control === 'leds') controls.ledsEnabled = message.params.enabled;
                if (message.params.control === 'headlights') controls.headlightsEnabled = message.params.enabled;
              }
              socket.send(
                JSON.stringify(
                  fail
                    ? { id: message.id, error: { code: 'unavailable', message: 'Control confirmation timed out' } }
                    : { id: message.id, result: controls }
                )
              );
            };
            return;
          }
          server.send(raw);
        });
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state =
            message.event === 'state.changed' ? message.data : message.result?.activeConnection ? message.result : null;
          if (state?.activeConnection) {
            state.activeConnection.refloatControls = { ...controls };
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
      const leds = page.getByRole('switch', { name: 'LEDs', exact: true });
      const headlights = page.getByRole('switch', { name: 'Headlights', exact: true });
      await leds.waitFor();
      assert.equal(await leds.getAttribute('aria-checked'), 'true');
      assert.equal(await headlights.getAttribute('aria-checked'), 'false');
      assert.ok((await leds.boundingBox())!.height >= 44);
      await leds.click({ button: 'right' });
      await leds.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
      await leds.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 2 });
      await leds.evaluate((el) =>
        el.addEventListener(
          'pointerdown',
          (event) => el.setAttribute('data-pointer', String((event as PointerEvent).pointerId)),
          { once: true }
        )
      );
      await leds.hover();
      await page.mouse.down();
      await leds.dispatchEvent('pointercancel', { pointerId: Number(await leds.getAttribute('data-pointer')) });
      await page.mouse.up();
      assert.equal(requests.length, 0);
      await leds.tap();
      await leds.getByText('Updating…').waitFor();
      assert.equal(requests.length, 1);
      assert.equal(await leds.getAttribute('aria-checked'), 'true');
      assert.equal(await leds.isDisabled(), true);
      assert.equal(await headlights.isDisabled(), true);
      assert.equal(await menu.getAttribute('aria-expanded'), 'true');
      complete!();
      await page.waitForFunction(
        () => document.querySelector('[role="switch"][aria-label="LEDs"]')?.getAttribute('aria-checked') === 'false'
      );
      await headlights.focus();
      await page.keyboard.press('Space');
      await headlights.getByText('Updating…').waitFor();
      assert.equal(requests.length, 2);
      complete!();
      await page.waitForFunction(
        () =>
          document.querySelector('[role="switch"][aria-label="Headlights"]')?.getAttribute('aria-checked') === 'true'
      );
      await leds.focus();
      await page.keyboard.press('Enter');
      await leds.getByText('Updating…').waitFor();
      assert.equal(requests.length, 3);
      complete!(true);
      await page.getByRole('alert').filter({ hasText: 'Control confirmation timed out' }).waitFor();
      assert.equal(await leds.getAttribute('aria-checked'), 'false');
      await page.waitForFunction(
        () => !document.querySelector('[role="switch"][aria-label="LEDs"]')?.hasAttribute('disabled')
      );
      await leds.dispatchEvent('click', { detail: 0 });
      await leds.getByText('Updating…').waitFor();
      assert.equal(requests.length, 4);
      complete!();
      await page.waitForFunction(
        () => document.querySelector('[role="switch"][aria-label="LEDs"]')?.getAttribute('aria-checked') === 'true'
      );
      await headlights.click();
      await headlights.getByText('Updating…').waitFor();
      assert.deepEqual(requests, [
        { control: 'leds', enabled: false },
        { control: 'headlights', enabled: true },
        { control: 'leds', enabled: true },
        { control: 'leds', enabled: true },
        { control: 'headlights', enabled: false },
      ]);
      complete!();
      await page.waitForFunction(
        () =>
          document.querySelector('[role="switch"][aria-label="Headlights"]')?.getAttribute('aria-checked') === 'false'
      );
      controls.headlightsEnabled = null;
      const unknown = page.getByRole('button', { name: 'Headlights', exact: true });
      await unknown.getByText('Unknown').waitFor();
      assert.equal(await unknown.isDisabled(), true);
      assert.equal(await headlights.count(), 0);
      controls.headlightsEnabled = false;
      await headlights.waitFor();
      for (const label of ['Disable Refloat…', 'Enable flywheel…']) {
        await page.getByRole('button', { name: label, exact: true }).tap();
        const dialog = page.getByRole('alertdialog');
        await dialog.waitFor();
        assert.equal(await menu.getAttribute('aria-expanded'), 'false');
        assert.equal(requests.length, 5);
        await dialog.getByRole('button', { name: 'Cancel', exact: true }).tap();
        await menu.tap();
      }
      const output = process.env.FLOAT_VIEW_SCREENSHOTS;
      for (const theme of ['light', 'dark'] as const) {
        await client.request('settings.update', {
          expectedRevision: client.currentState()!.settings.revision,
          patch: { themePreference: theme },
        });
        await page.waitForFunction((value) => document.documentElement.dataset.theme === value, theme);
        await page.goto(`${endpoint.origin}/#/settings`);
        await page.getByRole('group', { name: 'Theme', exact: true }).waitFor();
        await page.goto(`${endpoint.origin}/#/live`);
        await menu.tap();
        await leds.waitFor();
        if (output && name === 'chromium') {
          await mkdir(output, { recursive: true });
          await page.screenshot({ path: `${output}/live-menu-${theme}.png` });
        }
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
      }
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
