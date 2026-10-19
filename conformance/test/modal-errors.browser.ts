import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: failed modal actions stay open with visible, retryable errors`, { timeout: 90000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({
        viewport: { width: 390, height: 600 },
        hasTouch: true,
        reducedMotion: 'reduce',
      });
      page.setDefaultTimeout(5000);
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const errors: string[] = [];
      page.on('pageerror', (error) => errors.push(error.message));
      let failMethod: string | null = null;
      let succeedMethod: string | null = null;
      let unsavedDiscovery = false;
      let flywheel = false;
      let rideState: 'idle' | 'riding' = 'idle';
      const attempts: string[] = [];
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state =
            message.event === 'state.changed' ? message.data : message.result?.savedDevices ? message.result : null;
          if (unsavedDiscovery && state?.discoveredDevices) {
            for (const device of state.discoveredDevices) device.savedDeviceId = null;
          }
          if (state?.activeConnection) {
            state.activeConnection.controllerActions.shutdownAvailable = true;
            state.activeConnection.refloatControls.flywheelEnabled = flywheel;
          }
          if (state?.activeConnection?.latestSample) {
            Object.assign(state.activeConnection.latestSample, {
              rideState,
              footpadState: 'none',
              packageState: 'ready',
              speedKph: 0,
            });
          }
          socket.send(JSON.stringify(message));
        });
        socket.onMessage((raw) => {
          const request = JSON.parse(String(raw));
          if (request.method === failMethod || request.method === succeedMethod) {
            attempts.push(request.method);
            socket.send(
              JSON.stringify(
                request.method === failMethod
                  ? { id: request.id, error: { code: 'unavailable', message: 'Modal test failure' } }
                  : { id: request.id, result: { accepted: true } }
              )
            );
            return;
          }
          server.send(raw);
        });
      });
      const openAction = async (menu: string, action: string) => {
        await page.getByRole('button', { name: menu, exact: true }).tap();
        await page.getByRole('button', { name: action, exact: true }).tap();
      };
      const expectFailure = async (title: string, button: string, method: string) => {
        const dialog = page.getByRole('alertdialog', { name: title, exact: true });
        failMethod = method;
        const confirm = dialog.getByRole('button', { name: button, exact: true });
        const before = attempts.length;
        await confirm.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 2 });
        await confirm.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 2 });
        assert.equal(attempts.length, before);
        await confirm.tap();
        const alert = dialog.getByRole('alert');
        await alert.waitFor();
        assert.equal(await alert.textContent(), 'Modal test failure');
        const bounds = await alert.boundingBox();
        assert.ok(bounds && bounds.y >= 0 && bounds.y + bounds.height <= 600, `${title} error is outside the viewport`);
        assert.equal(await confirm.isEnabled(), true);
        assert.equal(attempts.length, before + 1);
        return dialog;
      };
      await page.goto(`${endpoint.origin}/#/sessions/session-1`);
      await openAction('Session actions', 'Delete session');
      let dialog = await expectFailure('Delete session?', 'Delete', 'session.delete');
      await dialog.getByRole('button', { name: 'Cancel', exact: true }).tap();
      await dialog.waitFor({ state: 'hidden' });
      await openAction('Session actions', 'Delete session');
      assert.equal(await page.getByRole('alertdialog').getByRole('alert').count(), 0);
      failMethod = null;
      await page.getByRole('alertdialog').getByRole('button', { name: 'Delete', exact: true }).focus();
      await page.keyboard.press('Enter');
      await page.getByRole('alertdialog').waitFor({ state: 'hidden' });

      await page.goto(`${endpoint.origin}/#/sessions/boards/saved-board`);
      await openAction('Board actions', 'Forget board');
      dialog = await expectFailure('Forget board?', 'Forget board', 'savedDevice.delete');
      await dialog.getByRole('button', { name: 'Cancel', exact: true }).tap();
      await dialog.waitFor({ state: 'hidden' });
      failMethod = null;

      await page.goto(`${endpoint.origin}/#/sessions`);
      await openAction('Session actions', 'Prune all');
      dialog = await expectFailure('Prune all sessions?', 'Preview', 'session.prune.preview');
      await dialog.getByRole('button', { name: 'Cancel', exact: true }).tap();
      await dialog.waitFor({ state: 'hidden' });
      await openAction('Session actions', 'Prune all');
      assert.equal(await page.getByRole('alertdialog').getByRole('alert').count(), 0);
      failMethod = null;
      await page.getByRole('alertdialog').getByRole('button', { name: 'Preview', exact: true }).tap();
      dialog = await expectFailure('Prune all sessions?', 'Prune all', 'session.prune.start');
      await dialog.getByRole('button', { name: 'Cancel', exact: true }).tap();
      await dialog.waitFor({ state: 'hidden' });
      failMethod = null;

      unsavedDiscovery = true;
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      await page.getByRole('navigation', { name: 'Primary' }).getByRole('link', { name: 'connect', exact: true }).tap();
      await page.getByRole('button', { name: /Float View Test VESC/ }).tap();
      dialog = await expectFailure('New board', 'Save and connect', 'device.connect');
      failMethod = null;
      await dialog.getByRole('button', { name: 'Save and connect', exact: true }).focus();
      await page.keyboard.press('Enter');
      await dialog.waitFor({ state: 'hidden' });
      await client.waitForState((state) => state.activeConnection?.state === 'ready');

      for (const action of [
        {
          menu: 'Disable Refloat…',
          title: 'Disable Refloat package?',
          button: 'Disable package',
          method: 'refloat.control.set',
        },
        {
          menu: 'Shut down controller',
          title: 'Shut down controller?',
          button: 'Shut down',
          method: 'controller.shutdown',
        },
        { menu: 'End ride', title: 'End ride?', button: 'End ride', method: 'session.end' },
      ]) {
        await openAction('Board actions', action.menu);
        dialog = await expectFailure(action.title, action.button, action.method);
        failMethod = null;
        succeedMethod = action.method;
        await dialog.getByRole('button', { name: action.button, exact: true }).focus();
        await page.keyboard.press('Enter');
        await dialog.waitFor({ state: 'hidden' });
        succeedMethod = null;
      }
      flywheel = true;
      await page.reload();
      dialog = await expectFailure('Flywheel mode is active', 'Disable', 'refloat.control.set');
      failMethod = null;
      succeedMethod = 'refloat.control.set';
      flywheel = false;
      await dialog.getByRole('button', { name: 'Disable', exact: true }).focus();
      await page.keyboard.press('Enter');
      await dialog.waitFor({ state: 'hidden' });
      succeedMethod = null;

      rideState = 'riding';
      await page.reload();
      await openAction('Board actions', 'Disconnect');
      dialog = await expectFailure('Disconnect board?', 'Disconnect', 'device.disconnect');
      failMethod = null;
      await dialog.getByRole('button', { name: 'Disconnect', exact: true }).focus();
      await page.keyboard.press('Enter');
      await dialog.waitFor({ state: 'hidden' });
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
