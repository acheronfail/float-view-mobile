import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { permissionInventory } from '../../frontend/backend-mock/permissions.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: permission inventory and bounded pointer/keyboard recovery`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002);
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
      assert.match(await location.innerText(), /recording is off/);
      await button.dispatchEvent('pointerdown', { button: 2, isPrimary: true, pointerId: 2 });
      await button.dispatchEvent('pointerup', { button: 2, isPrimary: true, pointerId: 2 });
      await button.dispatchEvent('pointerdown', { button: 0, isPrimary: true, pointerId: 3 });
      await button.dispatchEvent('pointercancel', { pointerId: 3 });
      await button.dispatchEvent('pointerup', { button: 0, isPrimary: true, pointerId: 3 });
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
      for (const platform of ['macos', 'android29', 'android30', 'android31'] as const) {
        rows = permissionInventory(platform);
        await page.reload();
        await location.waitFor();
        assert.equal(await page.locator('[data-permission]').count(), platform === 'android31' ? 3 : 2);
        assert.equal(
          await page.locator('[data-permission="bluetooth"]').count(),
          platform === 'android29' || platform === 'android30' ? 0 : 1
        );
        const recording = page.getByRole('group', { name: 'Record ride location', exact: true });
        await recording.getByRole('button', { name: 'On', exact: true }).tap();
        await page
          .getByText('Optional for board-only use. Ride location recording is on.', { exact: true })
          .first()
          .waitFor();
        await recording.getByRole('button', { name: 'Off', exact: true }).tap();
        await page
          .getByText('Optional for board-only use. Ride location recording is off.', { exact: true })
          .first()
          .waitFor();
      }
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
      assert.equal(requests, 3);
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
