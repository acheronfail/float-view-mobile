import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';
import type { RefloatControls } from '../../frontend/src/lib/backend/contract.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: live lighting boxes, holds, menu confirmations and pending state`, { timeout: 60_000 }, async () => {
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
      const settings = (await client.request('state.get', {})).settings;
      await client.request('settings.update', {
        expectedRevision: settings.revision,
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
                if (message.params.control === 'package') controls.packageEnabled = message.params.enabled;
                if (message.params.control === 'flywheel') controls.flywheelEnabled = message.params.enabled;
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
      const actionOverlay = page.getByRole('dialog', { name: 'Board actions', exact: true });
      await actionOverlay.waitFor();
      const detailed = actionOverlay.getByRole('button', { name: 'Detailed View', exact: true });
      const configureBoard = actionOverlay.getByRole('button', { name: 'Configure board', exact: true });
      assert.equal(await configureBoard.locator('.i-lucide-settings').count(), 1);
      assert.equal(await configureBoard.locator('.i-lucide-triangle-alert').count(), 1);
      for (const tile of await actionOverlay.locator('section button').all()) {
        assert.ok((await tile.locator('span[aria-hidden="true"]').count()) >= 1);
      }
      await page.getByRole('button', { name: 'Session…', exact: true }).waitFor();
      assert.deepEqual(await actionOverlay.getByRole('heading', { level: 3 }).allTextContents(), [
        'View',
        'Refloat',
        'Board',
        'Connection',
      ]);
      assert.equal(
        await actionOverlay
          .getByRole('button', { name: 'Done', exact: true })
          .evaluate((el) => el === document.activeElement),
        true
      );
      await page.keyboard.press('Shift+Tab');
      assert.equal(
        await actionOverlay
          .getByRole('button', { name: 'Disconnect', exact: true })
          .evaluate((el) => el === document.activeElement),
        true
      );
      await page.keyboard.press('Tab');
      assert.equal(
        await actionOverlay
          .getByRole('button', { name: 'Done', exact: true })
          .evaluate((el) => el === document.activeElement),
        true
      );
      await detailed.click({ button: 'right' });
      await detailed.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
      await detailed.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 2 });
      assert.equal(await menu.getAttribute('aria-expanded'), 'true');
      // Scroll gestures and cancelled taps must not activate a tile.
      for (const gesture of ['scroll', 'cancel']) {
        await detailed.evaluate((el) =>
          el.addEventListener(
            'pointerdown',
            (event) => {
              el.setAttribute('data-pointer', String((event as PointerEvent).pointerId));
            },
            { once: true }
          )
        );
        await detailed.hover();
        const point = (await detailed.boundingBox())!;
        await page.mouse.down();
        if (gesture === 'scroll') await page.mouse.move(point.x + point.width / 2, point.y + point.height / 2 + 30);
        else
          await detailed.dispatchEvent('pointercancel', {
            pointerId: Number(await detailed.getAttribute('data-pointer')),
          });
        await page.mouse.up();
        assert.equal(await menu.getAttribute('aria-expanded'), 'true');
      }
      await detailed.focus();
      await page.keyboard.press('Enter');
      await actionOverlay.waitFor({ state: 'hidden' });
      assert.equal(await menu.evaluate((el) => el === document.activeElement), true);
      await menu.tap();
      await actionOverlay.getByRole('button', { name: 'Normal View', exact: true }).tap();
      await actionOverlay.waitFor({ state: 'hidden' });
      await menu.tap();
      assert.equal(await page.getByRole('button', { name: 'LEDs', exact: true }).count(), 0);
      assert.equal(await page.getByRole('button', { name: 'Headlights', exact: true }).count(), 0);
      await page.keyboard.press('Escape');
      assert.equal(await menu.evaluate((el) => el === document.activeElement), true);
      const leds = page.getByRole('switch', { name: 'LEDs', exact: true });
      const headlights = page.getByRole('switch', { name: 'Headlights', exact: true });
      await leds.waitFor();
      assert.equal(await leds.getAttribute('aria-checked'), 'true');
      assert.equal(await headlights.getAttribute('aria-checked'), 'false');
      const assertBoxHeights = async () => {
        const heights = await page
          .locator('[aria-keyshortcuts="Shift+Enter"]')
          .evaluateAll((boxes) => boxes.map((box) => getComputedStyle(box).height));
        assert.deepEqual(heights, Array(5).fill('56px'));
      };
      await assertBoxHeights();
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
      const rowWidth = (await leds.boundingBox())!.width;
      await leds.tap();
      await leds.and(page.locator('[aria-busy="true"]')).waitFor();
      assert.equal(await leds.getAttribute('aria-label'), 'LEDs');
      await leds.getByText('Pending…').waitFor();
      await assertBoxHeights();
      assert.equal((await leds.boundingBox())!.width, rowWidth);
      assert.equal(requests.length, 1);
      assert.equal(await leds.getAttribute('aria-checked'), 'true');
      assert.equal(await leds.isDisabled(), true);
      assert.equal(await headlights.isDisabled(), true);
      assert.equal(await menu.getAttribute('aria-expanded'), 'false');
      complete!();
      await page.waitForFunction(
        () => document.querySelector('[role="switch"][aria-label="LEDs"]')?.getAttribute('aria-checked') === 'false'
      );
      await headlights.focus();
      await page.keyboard.press('Space');
      await headlights.and(page.locator('[aria-busy="true"]')).waitFor();
      assert.equal(await headlights.getAttribute('aria-label'), 'Headlights');
      assert.equal(await headlights.isDisabled(), true);
      assert.equal((await headlights.boundingBox())!.width, rowWidth);
      assert.equal(requests.length, 2);
      complete!();
      await page.waitForFunction(
        () =>
          document.querySelector('[role="switch"][aria-label="Headlights"]')?.getAttribute('aria-checked') === 'true'
      );
      await leds.focus();
      await page.keyboard.press('Enter');
      await leds.and(page.locator('[aria-busy="true"]')).waitFor();
      assert.equal(requests.length, 3);
      complete!(true);
      await page.getByRole('alert').filter({ hasText: 'Control confirmation timed out' }).waitFor();
      assert.equal(await leds.getAttribute('aria-checked'), 'false');
      await page.waitForFunction(
        () => document.querySelector('[role="switch"][aria-label="LEDs"]')?.getAttribute('aria-disabled') === 'false'
      );
      await leds.dispatchEvent('click', { detail: 0 });
      await leds.and(page.locator('[aria-busy="true"]')).waitFor();
      assert.equal(requests.length, 4);
      complete!();
      await page.waitForFunction(
        () => document.querySelector('[role="switch"][aria-label="LEDs"]')?.getAttribute('aria-checked') === 'true'
      );
      await headlights.click();
      await headlights.and(page.locator('[aria-busy="true"]')).waitFor();
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
      await assertBoxHeights();
      assert.equal(await unknown.getAttribute('aria-disabled'), 'true');
      assert.equal(await headlights.count(), 0);
      await unknown.focus();
      await page.keyboard.press('Shift+Enter');
      const picker = page.getByRole('dialog', { name: 'Choose a value' });
      await picker.waitFor();
      assert.equal(requests.length, 5);
      await page.keyboard.press('Escape');
      controls.headlightsEnabled = false;
      await headlights.waitFor();
      await headlights.hover();
      await page.mouse.down();
      await page.waitForTimeout(650);
      await page.mouse.up();
      await picker.waitFor();
      assert.equal(requests.length, 5, 'holding a light configures without toggling');
      await page.keyboard.press('Escape');
      const output = process.env.FLOAT_VIEW_SCREENSHOTS;
      for (const [label, title, confirmLabel, control] of [
        ['Disable Refloat…', 'Disable Refloat package?', 'Disable package', 'package'],
        ['Enable flywheel…', 'Enable flywheel?', 'Enable flywheel', 'flywheel'],
      ]) {
        const before: number = requests.length;
        await menu.tap();
        await page.getByRole('button', { name: label, exact: true }).tap();
        const dialog = page.getByRole('alertdialog', { name: title, exact: true });
        await dialog.waitFor();
        await actionOverlay.waitFor({ state: 'hidden' });
        assert.equal(await menu.getAttribute('aria-expanded'), 'false');
        assert.equal(requests.length, before);
        const cancel = dialog.getByRole('button', { name: 'Cancel', exact: true });
        const confirm = dialog.getByRole('button', { name: confirmLabel, exact: true });
        assert.equal(await cancel.evaluate((el) => el === document.activeElement), true);
        await page.keyboard.press('Shift+Tab');
        assert.equal(await confirm.evaluate((el) => el === document.activeElement), true);
        await page.keyboard.press('Tab');
        assert.equal(await cancel.evaluate((el) => el === document.activeElement), true);
        if (output && name === 'chromium') {
          await mkdir(output, { recursive: true });
          await page.screenshot({ path: `${output}/live-${control}-confirmation.png` });
        }
        await cancel.tap();
        await dialog.waitFor({ state: 'hidden' });
        assert.equal(requests.length, before);
        assert.equal(await menu.evaluate((el) => el === document.activeElement), true);
        await page.keyboard.press('Enter');
        const option = actionOverlay.getByRole('button', { name: label, exact: true });
        await option.focus();
        await page.keyboard.press('Enter');
        await dialog.waitFor();
        await page.keyboard.press('Escape');
        await dialog.waitFor({ state: 'hidden' });
        assert.equal(requests.length, before);
        // Failed requests preserve confirmed state; a retry submits only once.
        for (const fail of [true, false]) {
          await menu.tap();
          await page.getByRole('button', { name: label, exact: true }).tap();
          await confirm.tap();
          const working = dialog.getByRole('button', { name: 'Working…', exact: true });
          await working.waitFor();
          assert.equal(await working.isDisabled(), true);
          assert.equal(await cancel.isDisabled(), true);
          const expected = before + (fail ? 1 : 2);
          assert.equal(requests.length, expected);
          assert.deepEqual(requests.at(-1), { control, enabled: control === 'flywheel' });
          await page.keyboard.press('Escape');
          assert.equal(await dialog.isVisible(), true);
          await working.dispatchEvent('click', { detail: 0 });
          assert.equal(requests.length, expected);
          complete!(fail);
          await dialog.waitFor({ state: 'hidden' });
          if (fail) {
            await page.getByRole('alert').filter({ hasText: 'Control confirmation timed out' }).waitFor();
            assert.equal(controls.packageEnabled, true);
            assert.equal(controls.flywheelEnabled, false);
          } else if (control === 'package') {
            await menu.tap();
            await actionOverlay.getByRole('button', { name: 'Enable Refloat', exact: true }).waitFor();
            await page.keyboard.press('Escape');
            controls.packageEnabled = true;
          } else {
            const safety = page.getByRole('alertdialog', { name: 'Flywheel mode is active' });
            await safety.waitFor();
            assert.equal(await menu.isDisabled(), true);
            controls.flywheelEnabled = false;
            await safety.waitFor({ state: 'hidden' });
          }
        }
      }
      for (const theme of ['light', 'dark'] as const) {
        await client.request('settings.update', {
          expectedRevision: client.currentState()!.settings.revision,
          patch: { themePreference: theme },
        });
        await page.waitForFunction((value) => document.documentElement.dataset.theme === value, theme);
        await page.goto(`${endpoint.origin}/#/settings`);
        await page.getByRole('group', { name: 'Theme', exact: true }).waitFor();
        await page.goto(`${endpoint.origin}/#/live`);
        await leds.waitFor();
        await menu.tap();
        await actionOverlay.waitFor();
        if (output && name === 'chromium') {
          await mkdir(output, { recursive: true });
          await page.screenshot({ path: `${output}/live-actions-${theme}-mobile.png` });
          await page.setViewportSize({ width: 1280, height: 900 });
          await page.screenshot({ path: `${output}/live-actions-${theme}-desktop.png` });
          await page.setViewportSize({ width: 390, height: 844 });
        }
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
        // Small landscape screens scroll inside the overlay and keep dismissal available.
        await page.setViewportSize({ width: 568, height: 320 });
        await actionOverlay.getByRole('button', { name: 'Disconnect', exact: true }).scrollIntoViewIfNeeded();
        assert.equal(await actionOverlay.getByRole('button', { name: 'Done', exact: true }).isVisible(), true);
        await actionOverlay.getByRole('button', { name: 'Done', exact: true }).tap();
        assert.equal(await menu.getAttribute('aria-expanded'), 'false');
        await page.setViewportSize({ width: 390, height: 844 });
      }
      await menu.tap();
      controls.flywheelEnabled = true;
      await page.getByRole('alertdialog', { name: 'Flywheel mode is active' }).waitFor();
      await actionOverlay.waitFor({ state: 'hidden' });
      assert.equal(await menu.isDisabled(), true);
      assert.equal(await menu.getAttribute('aria-expanded'), 'false');
      await page.keyboard.press('Escape');
      assert.equal(await page.getByRole('alertdialog', { name: 'Flywheel mode is active' }).isVisible(), true);
      controls.flywheelEnabled = false;
      await page.getByRole('alertdialog', { name: 'Flywheel mode is active' }).waitFor({ state: 'hidden' });
      await menu.tap();
      await actionOverlay.waitFor();
      await page.keyboard.press('Escape');
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
