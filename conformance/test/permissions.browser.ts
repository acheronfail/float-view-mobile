import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { permissionInventory } from '../../frontend/backend-mock/permissions.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: permission inventory and bounded pointer/keyboard recovery`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({ hasTouch: true });
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      let rows = permissionInventory('ios');
      let requests = 0;
      let stale = false;
      let closeBackend = () => {};
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        closeBackend = () => server.close();
        socket.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (message.method === 'permissions.recover') {
            requests++;
            if (stale) {
              socket.send(
                JSON.stringify({
                  id: message.id,
                  error: { code: 'invalid_state', message: 'Permission action changed' },
                })
              );
              return;
            }
          }
          server.send(raw);
        });
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (message.result?.permissions) message.result.permissions = rows;
          if (message.data?.permissions) message.data.permissions = rows;
          socket.send(JSON.stringify(message));
        });
      });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.goto(`${endpoint.origin}/#/settings`);
      const location = page.locator('[data-permission="location"]');
      const button = location.getByRole('button', { name: 'Request access' });
      await button.waitFor();
      assert.match(
        await location.innerText(),
        /uses your location to record your ride route so you can review it on a map\. Route recording is optional/
      );
      await button.dispatchEvent('pointerdown', { button: 2, isPrimary: true, pointerId: 2 });
      await button.dispatchEvent('pointerup', { button: 2, isPrimary: true, pointerId: 2 });
      await button.dispatchEvent('pointerdown', { button: 0, isPrimary: true, pointerId: 3 });
      await button.dispatchEvent('pointercancel', { pointerId: 3 });
      await button.dispatchEvent('pointerup', { button: 0, isPrimary: true, pointerId: 3 });
      await button.dispatchEvent('pointerdown', { button: 0, isPrimary: false, pointerId: 4 });
      await button.dispatchEvent('pointerup', { button: 0, isPrimary: false, pointerId: 4 });
      await button.dispatchEvent('pointerdown', { button: 0, isPrimary: true, pointerId: 5, clientX: 20, clientY: 20 });
      await button.dispatchEvent('pointermove', { pointerId: 5, clientX: 20, clientY: 50 });
      await button.dispatchEvent('pointerup', { button: 0, isPrimary: true, pointerId: 5, clientX: 20, clientY: 50 });
      assert.equal(requests, 0);
      await button.tap();
      await page.waitForFunction(
        () => !document.querySelector<HTMLButtonElement>('[data-permission="location"] button')?.disabled
      );
      assert.equal(requests, 1);
      await button.focus();
      await page.keyboard.press('Enter');
      await page.waitForFunction(
        () => !document.querySelector<HTMLButtonElement>('[data-permission="location"] button')?.disabled
      );
      assert.equal(requests, 2);
      stale = true;
      await button.dispatchEvent('click', { detail: 0 });
      await page.getByRole('alert').filter({ hasText: 'Permission action changed' }).waitFor();
      assert.equal(requests, 3);
      for (const platform of ['macos', 'android29', 'android30', 'android31', 'android33', 'android36'] as const) {
        rows = permissionInventory(platform);
        await page.reload();
        await location.waitFor();
        assert.equal(await page.locator('[data-permission]').count(), rows.length);
        assert.equal(
          await page.locator('[data-permission="bluetooth"]').count(),
          platform === 'android29' || platform === 'android30' ? 0 : 1
        );
        assert.equal(await page.getByText(/Optional for board-only use/).count(), 0);
      }
      const notifications = page.locator('[data-permission="notifications"]');
      const liveUpdates = page.locator('[data-permission="liveUpdates"]');
      assert.match(await notifications.innerText(), /Optional/);
      assert.doesNotMatch(await notifications.innerText(), /Ride location recording/);
      assert.match(await liveUpdates.innerText(), /Allow Notifications first/);
      assert.equal(await liveUpdates.getByRole('button').count(), 0);
      const notifyRequest = notifications.getByRole('button', { name: 'Request access' });
      await notifyRequest.dispatchEvent('pointerdown', { button: 2, isPrimary: true, pointerId: 7 });
      await notifyRequest.dispatchEvent('pointerup', { button: 2, isPrimary: true, pointerId: 7 });
      assert.equal(requests, 3);
      await notifyRequest.tap();
      await page.getByRole('alert').filter({ hasText: 'Permission action changed' }).waitFor();
      assert.equal(requests, 4);
      rows = rows.map((row) =>
        row.permission === 'notifications'
          ? { ...row, status: 'granted', action: 'openSettings', guidance: 'notificationSettings' }
          : row.permission === 'liveUpdates'
            ? { ...row, action: 'openSettings', guidance: 'liveUpdateSettings' }
            : row
      );
      await page.reload();
      await liveUpdates.getByRole('button', { name: 'Open settings' }).focus();
      await page.keyboard.press('Enter');
      await page.getByRole('alert').filter({ hasText: 'Permission action changed' }).waitFor();
      assert.equal(requests, 5);
      rows[1] = {
        ...rows[1]!,
        status: 'limited',
        action: 'openSettings',
        accuracy: 'approximate',
        scope: 'whileInUse',
      };
      await page.reload();
      await location.getByText('Approximate location', { exact: true }).waitFor();
      rows[1] = { ...rows[1]!, pending: true };
      await page.reload();
      await location.getByRole('button').waitFor();
      assert.equal(await location.getByRole('button').isDisabled(), true);
      assert.equal(requests, 5);
      rows[1] = { ...rows[1]!, pending: false };
      await page.reload();
      await location.getByRole('button').waitFor();
      closeBackend();
      await page.getByRole('button', { name: 'Reconnect', exact: true }).waitFor();
      assert.equal(await location.getByRole('button').isDisabled(), true);
      await page.getByRole('button', { name: 'Reconnect', exact: true }).click();
      await page.waitForFunction(
        () => !document.querySelector<HTMLButtonElement>('[data-permission="location"] button')?.disabled
      );
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}

for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: Bluetooth off pauses discovery and preserves sessions`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({ hasTouch: true });
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      let poweredOff = true;
      let scans = 0;
      let resume = () => {};
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        let lastState: import('../../frontend/src/lib/backend/contract.ts').AppState;
        resume = () => {
          poweredOff = false;
          socket.send(
            JSON.stringify({
              event: 'state.changed',
              data: {
                ...lastState,
                revision: lastState.revision + 1,
                bluetooth: { ...lastState.bluetooth, radio: 'powered_on', scanning: false },
              },
            })
          );
        };
        socket.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (message.method === 'bluetooth.scan') scans++;
          server.send(raw);
        });
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state = message.result?.bluetooth ? message.result : message.data?.bluetooth ? message.data : null;
          if (state) {
            state.bluetooth.radio = poweredOff ? 'powered_off' : 'powered_on';
            state.permissions = permissionInventory('ios').map((row) =>
              row.permission === 'bluetooth'
                ? {
                    ...row,
                    status: 'granted',
                    servicesEnabled: !poweredOff,
                    guidance: poweredOff ? 'servicesDisabled' : 'none',
                    action: null,
                  }
                : row
            );
            lastState = state;
          }
          socket.send(JSON.stringify(message));
        });
      });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.goto(`${endpoint.origin}/#/`);
      await page.getByRole('heading', { name: 'Bluetooth is off' }).waitFor();
      assert.match(await page.locator('main').innerText(), /Turn on Bluetooth/);
      assert.doesNotMatch(
        await page.locator('main').innerText(),
        /Scanning…|Looking for VESC|Backend service is unavailable/
      );
      await page.waitForTimeout(1200);
      assert.equal(scans, 0);
      await page.getByRole('link', { name: 'sessions', exact: true }).click();
      assert.equal(await page.getByRole('alert').count(), 0);
      await page.getByRole('link', { name: 'settings', exact: true }).click();
      const bluetooth = page.locator('[data-permission="bluetooth"]');
      await bluetooth.getByText('Bluetooth is off.', { exact: true }).waitFor();
      assert.match(await bluetooth.innerText(), /Bluetooth permission is already allowed/);
      await page.getByRole('link', { name: 'connect', exact: true }).click();
      await page.getByRole('heading', { name: 'Bluetooth is off' }).waitFor();
      resume();
      await page.getByRole('heading', { name: 'Bluetooth is off' }).waitFor({ state: 'hidden' });
      await page.waitForTimeout(1500);
      assert.ok(scans > 0, 'discovery resumes after Bluetooth is enabled');
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}

for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: first-use Bluetooth permission, readiness, and scan retry`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002, { safetyDisclaimerAccepted: true });
    const endpoint = getMockBootstrap(port);
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({ hasTouch: true });
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      let bluetooth: import('../../frontend/src/lib/backend/contract.ts').BluetoothState = {
        radio: 'unknown',
        scanPermission: 'not_determined',
        connectPermission: 'not_determined',
        scanning: false,
      };
      let scans = 0;
      let requests = 0;
      let failScan = true;
      let grant = () => {};
      let ready = () => {};
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        let lastState: import('../../frontend/src/lib/backend/contract.ts').AppState;
        let revision = 100;
        const update = () => {
          socket.send(
            JSON.stringify({ event: 'state.changed', data: { ...lastState, revision: ++revision, bluetooth } })
          );
        };
        ready = () => {
          bluetooth = { ...bluetooth, radio: 'powered_on' };
          update();
        };
        socket.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (message.method === 'bluetooth.scan') {
            scans++;
            if (bluetooth.scanPermission === 'not_determined') {
              socket.send(
                JSON.stringify({
                  id: message.id,
                  error: { code: 'permission_required', message: 'Permission required' },
                })
              );
            } else if (failScan) {
              socket.send(
                JSON.stringify({
                  id: message.id,
                  error: { code: 'unavailable', message: 'Backend service is unavailable' },
                })
              );
            } else {
              bluetooth = { ...bluetooth, scanning: true };
              update();
              socket.send(JSON.stringify({ id: message.id, result: { accepted: true } }));
            }
            return;
          }
          if (message.method === 'bluetooth.requestPermission') {
            requests++;
            grant = () => {
              bluetooth = { ...bluetooth, scanPermission: 'granted', connectPermission: 'granted' };
              update();
              socket.send(JSON.stringify({ id: message.id, result: bluetooth }));
            };
            return;
          }
          server.send(raw);
        });
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state = message.result?.permissions
            ? message.result
            : message.event === 'state.changed'
              ? message.data
              : null;
          if (state) {
            state.bluetooth = bluetooth;
            state.revision = ++revision;
            state.discoveredDevices = [];
            state.permissions = permissionInventory('macos');
            lastState = state;
          }
          socket.send(JSON.stringify(message));
        });
      });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.goto(endpoint.origin);
      await page.getByRole('heading', { name: 'Waiting for Bluetooth access' }).waitFor();
      assert.equal(requests, 1);
      assert.equal(scans, 1);
      assert.equal(await page.getByText('Ready to scan', { exact: true }).count(), 0);
      grant();
      await page.getByRole('heading', { name: 'Checking Bluetooth' }).waitFor();
      assert.equal(scans, 1, 'Do not scan before the radio is ready');
      ready();
      const retry = page.getByRole('button', { name: 'Try scanning again' });
      await retry.waitFor();
      await page.getByText('Scan unavailable', { exact: true }).waitFor();
      assert.equal(await page.getByText('Backend service is unavailable', { exact: false }).count(), 0);
      assert.equal(await page.getByText('Keep your board nearby and powered on.', { exact: true }).count(), 0);
      const before = scans;
      await retry.dispatchEvent('pointerdown', { button: 2, isPrimary: true, pointerId: 2 });
      await retry.dispatchEvent('pointerup', { button: 2, isPrimary: true, pointerId: 2 });
      await retry.dispatchEvent('pointerdown', { button: 0, isPrimary: true, pointerId: 3 });
      await retry.dispatchEvent('pointercancel', { pointerId: 3 });
      await retry.dispatchEvent('pointerup', { button: 0, isPrimary: true, pointerId: 3 });
      assert.equal(scans, before);
      await retry.tap();
      await page.waitForFunction(() => !document.querySelector<HTMLButtonElement>('[role="alert"] button')?.disabled);
      assert.equal(scans, before + 1);
      failScan = false;
      await retry.focus();
      await page.keyboard.press('Enter');
      await page.getByText('Looking for VESC devices…', { exact: true }).waitFor();
      assert.equal(scans, before + 2);
      assert.equal(requests, 1, 'Permission should not be requested again after approval');
    } finally {
      await browser.close();
      await stopMockServer();
    }
  });
}
