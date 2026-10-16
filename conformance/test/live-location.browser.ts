import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import type { LocationState } from '../../frontend/src/lib/backend/contract.ts';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
const longName = 'The Extremely Long Named Trail Board — Weekend Adventure Edition';
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: ride location preserves the live layout with long board names`, { timeout: 90_000 }, async () => {
    const port = await startMockServer(11002);
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
      const page = await browser.newPage({
        viewport: { width: 390, height: 844 },
        hasTouch: true,
        reducedMotion: 'reduce',
      });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const errors: string[] = [];
      const methods: string[] = [];
      page.on('pageerror', (e) => errors.push(e.message));
      let boardName = 'Trail Board';
      let override: LocationState | null = null;
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => {
          const request = JSON.parse(String(raw));
          if (request.method) methods.push(request.method);
          // Model the OS permission result; the deterministic mock leaves permissions unchanged.
          if (request.method === 'location.requestPermission') {
            override = { status: 'authorized', permissionAction: null, backgroundCapture: 'available', reason: null };
            socket.send(JSON.stringify({ id: request.id, result: override }));
            return;
          }
          if (request.method === 'location.openSettings') {
            socket.send(JSON.stringify({ id: request.id, result: override }));
            return;
          }
          server.send(raw);
        });
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state =
            message.event === 'state.changed' ? message.data : message.result?.activeConnection ? message.result : null;
          if (state?.activeConnection) {
            state.activeConnection.displayName = boardName;
            if (override) state.location = override;
          }
          socket.send(JSON.stringify(message));
        });
      });
      await page.goto(`${endpoint.origin}/#/live`);
      const heading = page.getByRole('heading', { level: 1 });
      const location = page.getByRole('button', { name: /^Ride location:/ });
      const menu = page.getByRole('button', { name: 'Board actions', exact: true });
      assert.equal(await menu.evaluate((el) => el.classList.contains('text-warning')), false);
      const panel = page.getByRole('dialog', { name: 'Ride location', exact: true });
      const gauge = page.getByRole('group', { name: 'Live speed and drive load gauge' });
      await gauge.waitFor();
      assert.equal(await panel.isVisible(), false, 'the location panel never opens automatically');
      assert.equal(await location.getAttribute('aria-label'), 'Ride location: off');
      for (const token of ['text-warning', 'border-warning/40', 'bg-warning/10']) {
        assert.equal(await location.evaluate((el, value) => el.classList.contains(value), token), true);
      }
      assert.equal(await page.getByText('Record this ride location?').count(), 0);

      for (const viewport of [
        { width: 320, height: 640 },
        { width: 390, height: 844 },
        { width: 768, height: 1024 },
        { width: 1280, height: 900 },
      ]) {
        await page.setViewportSize(viewport);
        boardName = 'Trail Board';
        await page.waitForFunction((value) => document.querySelector('h1')?.textContent === value, boardName);
        const before = (await gauge.boundingBox())!;
        const headerHeight = await heading
          .locator('xpath=ancestor::header')
          .evaluate((el) => el.getBoundingClientRect().height);
        for (const title of [longName, 'ExtremelyLongBoardNameWithoutAnySpacesToTestTruncationAndButtonVisibility']) {
          boardName = title;
          await page.waitForFunction((value) => document.querySelector('h1')?.textContent === value, boardName);
          const after = (await gauge.boundingBox())!;
          assert.deepEqual(after, before, 'the board name cannot resize or shift the gauge');
          const headingBox = (await heading.boundingBox())!;
          const locationBox = (await location.boundingBox())!;
          const menuBox = (await menu.boundingBox())!;
          assert.ok(headingBox.x + headingBox.width <= locationBox.x);
          assert.equal(locationBox.width, 44);
          assert.equal(menuBox.width, 44);
          assert.ok(menuBox.x + menuBox.width <= viewport.width);
          assert.equal(
            await heading.locator('xpath=ancestor::header').evaluate((el) => el.getBoundingClientRect().height),
            headerHeight
          );
          assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
        }
      }
      if (name === 'chromium') {
        await page.setViewportSize({ width: 320, height: 640 });
        await page.screenshot({ path: '/tmp/float-view-location-narrow.png' });
      }
      boardName = longName;
      await page.setViewportSize({ width: 390, height: 844 });
      await page.waitForFunction((value) => document.querySelector('h1')?.textContent === value, boardName);
      const originalGauge = (await gauge.boundingBox())!;
      if (name === 'chromium') await page.screenshot({ path: '/tmp/float-view-location-long-name.png' });

      // Ignore secondary pointers and cancelled primary gestures.
      await location.click({ button: 'right' });
      await location.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
      await location.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 2 });
      await location.dispatchEvent('pointerdown', { button: 0, isPrimary: true, pointerId: 1 });
      await location.dispatchEvent('pointercancel', { pointerId: 1 });
      await location.dispatchEvent('pointerup', { button: 0, isPrimary: true, pointerId: 1 });
      assert.equal(await panel.isVisible(), false);
      await location.tap();
      await panel.waitFor();
      assert.deepEqual(await gauge.boundingBox(), originalGauge, 'opening a panel cannot reflow the gauge');
      await panel.getByText('Recording off', { exact: true }).waitFor();
      if (name === 'chromium') await page.screenshot({ path: '/tmp/float-view-location-panel.png' });
      await panel.getByRole('button', { name: 'Done' }).focus();
      await page.keyboard.press('Shift+Tab');
      assert.equal(
        await panel.getByRole('button', { name: 'Enable' }).evaluate((el) => el === document.activeElement),
        true
      );
      await page.keyboard.press('Tab');
      assert.equal(
        await panel.getByRole('button', { name: 'Done' }).evaluate((el) => el === document.activeElement),
        true
      );
      await page.keyboard.press('Escape');
      await panel.waitFor({ state: 'hidden' });
      assert.equal(await location.evaluate((el) => el === document.activeElement), true);
      for (const key of ['Enter', 'Space']) {
        await location.focus();
        await page.keyboard.press(key);
        await panel.waitFor();
        await page.keyboard.press('Escape');
        await panel.waitFor({ state: 'hidden' });
      }
      await location.tap();
      await panel.getByRole('button', { name: 'Enable', exact: true }).tap();
      await panel.getByText('Recording enabled', { exact: true }).waitFor();
      assert.equal(methods.filter((method) => method === 'settings.update').length, 1);
      assert.equal(methods.filter((method) => method === 'location.requestPermission').length, 1);
      await location.waitFor({ state: 'detached' });
      await page.keyboard.press('Escape');
      await panel.waitFor({ state: 'hidden' });
      assert.equal(await menu.evaluate((el) => el === document.activeElement), true);
      assert.deepEqual(await gauge.boundingBox(), originalGauge, 'enabling location cannot resize the gauge');
      await menu.tap();
      assert.equal(await page.getByRole('button', { name: 'Ride location…', exact: true }).count(), 0);
      await page.keyboard.press('Escape');

      override = {
        status: 'notDetermined',
        permissionAction: 'requestPermission',
        backgroundCapture: 'unavailable',
        reason: 'Allow location access to record your route.',
      };
      await location.waitFor();
      await location.tap();
      await panel.getByRole('button', { name: 'Allow', exact: true }).waitFor();
      await panel.getByRole('button', { name: 'Allow', exact: true }).focus();
      await page.keyboard.press('Enter');
      await panel.getByText('Recording enabled', { exact: true }).waitFor();
      assert.equal(methods.filter((method) => method === 'location.requestPermission').length, 2);
      await location.waitFor({ state: 'detached' });
      await page.keyboard.press('Escape');
      await panel.waitFor({ state: 'hidden' });

      override = {
        status: 'denied',
        permissionAction: 'openSettings',
        backgroundCapture: 'unavailable',
        reason: 'Enable location in Settings to record your route.',
      };
      await page.getByRole('button', { name: 'Ride location: unavailable', exact: true }).waitFor();
      assert.match((await location.getAttribute('class'))!, /text-warning/);
      await menu.tap();
      await page.getByRole('button', { name: 'Ride location…', exact: true }).tap();
      await panel.waitFor();
      await panel.getByText(override.reason!, { exact: true }).waitFor();
      await panel.getByRole('button', { name: 'Settings', exact: true }).tap();
      await page.waitForFunction(() => !document.querySelector('dialog[open] button:disabled'));
      assert.equal(methods.filter((method) => method === 'location.openSettings').length, 1);
      if (name === 'chromium') await page.screenshot({ path: '/tmp/float-view-location-blocked.png' });
      await page.keyboard.press('Escape');
      await panel.waitFor({ state: 'hidden' });
      assert.equal(await menu.evaluate((el) => el === document.activeElement), true);

      override = {
        status: 'restricted',
        permissionAction: null,
        backgroundCapture: 'unavailable',
        reason: 'Location access is restricted on this device.',
      };
      await location.tap();
      await panel.getByText(override.reason!, { exact: true }).waitFor();
      assert.equal(await panel.getByRole('button').count(), 1, 'no unsupported recovery action is offered');
      await page.keyboard.press('Escape');
      await panel.waitFor({ state: 'hidden' });
      await page.setViewportSize({ width: 1280, height: 900 });
      if (name === 'chromium') await page.screenshot({ path: '/tmp/float-view-location-desktop.png' });
      await menu.tap();
      await page.getByRole('button', { name: 'Detailed View', exact: true }).tap();
      await location.tap();
      await panel.waitFor();
      await page.keyboard.press('Escape');
      await panel.waitFor({ state: 'hidden' });
      await client.request('device.disconnect', {});
      await client.waitForState((s) => !s.activeConnection);
      await client.request('device.connect', {
        deviceId: scanned.discoveredDevices[0]!.deviceId,
        trackingMode: 'guest',
      });
      await client.waitForState((s) => s.activeConnection?.state === 'ready');
      await page.goto(`${endpoint.origin}/#/live`);
      await gauge.waitFor();
      assert.equal(await location.count(), 0, 'guests have no ride-location header control');
      const guestWarning = 'Guest connection — live data will not be saved or tracked.';
      assert.equal(await page.getByText(guestWarning, { exact: true }).isVisible(), false);
      assert.equal(await menu.getAttribute('title'), guestWarning);
      assert.equal(await menu.evaluate((el) => el.classList.contains('text-warning')), true);
      await menu.click({ button: 'right' });
      await menu.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
      await menu.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 2 });
      assert.equal(await menu.getAttribute('aria-expanded'), 'false');
      const actions = page.getByRole('dialog', { name: 'Board actions', exact: true });
      for (const activation of ['touch', 'keyboard']) {
        if (activation === 'touch') await menu.tap();
        else {
          await menu.focus();
          await page.keyboard.press('Enter');
        }
        await actions.waitFor();
        const warning = actions.getByRole('status');
        assert.equal(await warning.textContent().then((text) => text?.trim()), guestWarning);
        const warningBounds = (await warning.boundingBox())!;
        const groupBounds = (await actions.locator('section').first().boundingBox())!;
        assert.ok(warningBounds.y + warningBounds.height <= groupBounds.y, 'guest warning precedes action groups');
        assert.equal(await page.getByRole('button', { name: 'Ride location…', exact: true }).count(), 0);
        await page.keyboard.press('Escape');
        await actions.waitFor({ state: 'hidden' });
        assert.equal(await menu.evaluate((el) => el === document.activeElement), true);
      }
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
