import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit, type Page } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
async function nativeBack(page: Page) {
  assert.equal(
    await page.evaluate(() => window.dispatchEvent(new Event('floatview:back', { cancelable: true }))),
    false
  );
}

for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: connected navigation, stable returns and guarded dismissal`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(0, { safetyDisclaimerAccepted: true });
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
      const ready = await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const connection = ready.activeConnection!;
      const boardHref = `#/sessions/boards/${encodeURIComponent(connection.savedDeviceId!)}`;
      const page = await browser.newPage({
        hasTouch: true,
        reducedMotion: 'reduce',
        viewport: { width: 390, height: 844 },
      });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const errors: string[] = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      let failUpdates = false;
      let presentAsBms = false;
      let updateRequests = 0;
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => {
          const request = JSON.parse(String(raw));
          if (request.method === 'savedDevice.update' || request.method === 'boardProfile.update') {
            updateRequests++;
            if (failUpdates) {
              socket.send(
                JSON.stringify({
                  id: request.id,
                  error: { code: 'internal_error', message: 'Navigation test save failure' },
                })
              );
              return;
            }
          }
          server.send(raw);
        });
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state =
            message.event === 'state.changed' ? message.data : message.result?.bluetooth ? message.result : null;
          if (presentAsBms && state?.activeConnection) {
            Object.assign(state.activeConnection, {
              deviceKind: 'bms',
              state: 'ready',
              activeSession: null,
              latestSample: null,
              firmware: null,
              controllerConnection: null,
              controllerCandidates: [],
              refloatControls: null,
              refloatConfiguration: null,
              bms: { status: 'attached', connection: { kind: 'direct' }, candidates: [], latestSnapshot: null },
            });
          }
          socket.send(JSON.stringify(message));
        });
      });
      await page.goto(`${endpoint.origin}/#/live`);
      const tabs = page.getByRole('navigation', { name: 'Primary' });
      const boardTab = tabs.getByRole('link', { name: 'board', exact: true });
      const settingsTab = tabs.getByRole('link', { name: 'settings', exact: true });
      const liveMenu = page.getByRole('button', { name: 'Board actions', exact: true });
      await liveMenu.waitFor();
      await page.keyboard.press('Escape');
      assert.equal(await tabs.count(), 0, 'Live reserves the full screen without tabs');
      await liveMenu.tap();
      const backToApp = page
        .getByRole('dialog', { name: 'Board actions', exact: true })
        .getByRole('button', { name: 'Go to Sessions', exact: true });
      await backToApp.dispatchEvent('pointerdown', { pointerId: 70, isPrimary: false, button: 0 });
      await backToApp.dispatchEvent('pointerup', { pointerId: 70, isPrimary: false, button: 0 });
      assert.match(page.url(), /#\/live$/);
      await backToApp.tap();
      await page.waitForURL(/#\/sessions$/);
      await boardTab.waitFor();
      await settingsTab.dispatchEvent('pointerdown', { pointerId: 71, isPrimary: false, button: 0 });
      await settingsTab.dispatchEvent('pointerup', { pointerId: 71, isPrimary: false, button: 0 });
      assert.match(page.url(), /#\/sessions$/);
      const settingsBox = (await settingsTab.boundingBox())!;
      await page.mouse.move(settingsBox.x + settingsBox.width / 2, settingsBox.y + settingsBox.height / 2);
      await page.mouse.down();
      await settingsTab.dispatchEvent('pointercancel', { pointerId: 1 });
      await page.mouse.up();
      assert.match(page.url(), /#\/sessions$/);
      await settingsTab.tap();
      await page.waitForURL(/#\/settings$/);
      assert.equal((await client.request('state.get', {})).activeConnection?.deviceId, connection.deviceId);
      await boardTab.focus();
      await page.keyboard.press('Enter');
      await page.waitForURL(/#\/live$/);
      assert.equal(await tabs.count(), 0);
      await nativeBack(page);
      await page.waitForURL(/#\/sessions$/);
      await page.getByRole('button', { name: /Current ride/ }).tap();
      await page.getByRole('link', { name: 'Sessions', exact: true }).waitFor();
      await nativeBack(page);
      await page.waitForURL(/#\/sessions$/);

      const boardLink = page.locator(`a[href="${boardHref}"]`);
      await boardLink.focus();
      await page.keyboard.press('Enter');
      const rideLink = page.locator(
        `a[href="#/sessions/${encodeURIComponent(connection.activeSession!.sessionId)}?board=${encodeURIComponent(connection.savedDeviceId!)}"]`
      );
      await rideLink.tap();
      await page.getByRole('link', { name: 'Board sessions', exact: true }).tap();
      await page.waitForURL((url) => url.hash === boardHref);
      const menu = page.getByRole('button', { name: 'Board actions', exact: true });
      await menu.tap();
      await page.keyboard.press('Escape');
      assert.equal(await menu.getAttribute('aria-expanded'), 'false');
      assert.equal(await menu.evaluate((el) => el === document.activeElement), true);
      await menu.tap();
      await nativeBack(page);
      assert.equal(await menu.getAttribute('aria-expanded'), 'false');
      await menu.tap();
      await page.getByRole('button', { name: 'Configure board', exact: true }).tap();
      const panel = page.getByRole('dialog', { name: 'Board configuration', exact: true });
      const displayName = panel.getByRole('textbox', { name: 'Display name', exact: true });
      await displayName.fill('Unsaved history edit');
      failUpdates = true;
      await panel.getByRole('button', { name: 'Done', exact: true }).tap();
      await panel.getByRole('alert').filter({ hasText: 'Navigation test save failure' }).waitFor();
      assert.equal(await panel.isVisible(), true, 'Done keeps failed edits open');
      assert.equal(await displayName.inputValue(), 'Unsaved history edit');
      await page.keyboard.press('Escape');
      assert.equal(await panel.isVisible(), true, 'Escape uses the same save guard');
      await nativeBack(page);
      assert.equal(await panel.isVisible(), true, 'system Back uses the same save guard');
      assert.equal(updateRequests, 1, 'dismissal does not double-submit failed edits');
      await panel.getByRole('button', { name: 'Close without saving failed changes' }).tap();
      await panel.waitFor({ state: 'hidden' });
      await menu.tap();
      await page.getByRole('button', { name: 'Configure board', exact: true }).tap();
      await panel.getByRole('spinbutton', { name: 'Preferred BMS CAN ID (optional)' }).fill('11');
      await panel.getByRole('button', { name: 'Done', exact: true }).tap();
      await panel.getByRole('alert').filter({ hasText: 'Navigation test save failure' }).waitFor();
      await nativeBack(page);
      assert.equal(await panel.isVisible(), true, 'history profile failures also prevent dismissal');
      await panel.getByRole('button', { name: 'Close without saving failed changes' }).tap();
      await panel.waitFor({ state: 'hidden' });
      failUpdates = false;
      await page.getByRole('link', { name: 'Sessions', exact: true }).tap();
      await boardTab.tap();
      await page.waitForURL(/#\/live$/);
      failUpdates = true;
      await menu.tap();
      await page
        .getByRole('dialog', { name: 'Board actions', exact: true })
        .getByRole('button', { name: 'Configure board', exact: true })
        .tap();
      await displayName.fill('Unsaved live edit');
      await panel.getByRole('button', { name: 'Done', exact: true }).tap();
      await panel.getByRole('alert').filter({ hasText: 'Navigation test save failure' }).waitFor();
      await page.keyboard.press('Escape');
      assert.equal(await panel.isVisible(), true, 'Live uses the same guard for failed name saves');
      await panel.getByRole('button', { name: 'Close without saving failed changes' }).tap();
      await panel.waitFor({ state: 'hidden' });
      failUpdates = false;
      await menu.tap();
      await page
        .getByRole('dialog', { name: 'Board actions', exact: true })
        .getByRole('button', { name: 'Session…', exact: true })
        .tap();
      const backToBoard = page.getByRole('link', { name: 'Back to board', exact: true });
      await backToBoard.waitFor();
      await page.reload();
      await backToBoard.waitFor();
      await client.request('device.disconnect', {});
      await client.waitForState((s) => !s.activeConnection);
      await page.getByRole('heading', { name: 'Current ride', exact: true }).waitFor({ state: 'hidden' });
      assert.equal(await backToBoard.getAttribute('href'), '#/live', 'completion preserves the entry destination');
      await backToBoard.tap();
      await page.waitForURL(/#\/$/);
      await tabs.getByRole('link', { name: 'connect', exact: true }).waitFor();
      presentAsBms = true;
      await client.request('device.connect', { deviceId: connection.deviceId, trackingMode: 'guest' });
      await page.waitForURL(/#\/bms$/);
      assert.equal(await tabs.count(), 0, 'standalone BMS also reserves the full screen');
      const bmsMenu = page.getByRole('button', { name: 'BMS actions', exact: true });
      await bmsMenu.tap();
      await page.getByRole('button', { name: 'Go to Sessions', exact: true }).tap();
      await page.waitForURL(/#\/sessions$/);
      await boardTab.focus();
      await page.keyboard.press('Enter');
      await page.waitForURL(/#\/bms$/);
      await bmsMenu.focus();
      await page.keyboard.press('Enter');
      await page.keyboard.press('Escape');
      assert.equal(await bmsMenu.getAttribute('aria-expanded'), 'false');
      await nativeBack(page);
      await page.waitForURL(/#\/sessions$/);
      assert.equal(
        (await client.request('state.get', {})).activeConnection?.deviceId,
        connection.deviceId,
        'menu navigation preserves the BMS connection'
      );
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      await client.close();
      await stopMockServer();
    }
  });
}
