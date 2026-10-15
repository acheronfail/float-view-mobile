import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';
const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: announcement settings and guarded activation`, { timeout: 60_000 }, async () => {
    const port = await startMockServer(11002);
    const endpoint = getMockBootstrap(port);
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const browser = await engine.launch();
    try {
      const page = await browser.newPage({ hasTouch: true, reducedMotion: 'reduce' });
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      const requests: string[] = [];
      const errors: string[] = [];
      page.on('pageerror', (e) => errors.push(e.message));
      page.on('websocket', (socket) =>
        socket.on('framesent', (frame) => {
          try {
            requests.push(JSON.parse(String(frame.payload)).method);
          } catch {}
        })
      );
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.goto(`${endpoint.origin}/#/settings`);
      const speed = page.getByRole('group', { name: 'Speed announcements', exact: true });
      await speed.getByRole('button', { name: 'On', exact: true }).waitFor();
      await speed.getByRole('button', { name: 'On', exact: true }).click({ button: 'right' });
      assert.equal(await speed.getByRole('button', { name: 'Off', exact: true }).getAttribute('aria-pressed'), 'true');
      await speed.getByRole('button', { name: 'On', exact: true }).tap();
      await page
        .getByRole('group', { name: 'Load announcements', exact: true })
        .getByRole('button', { name: 'On', exact: true })
        .focus();
      await page.keyboard.press('Enter');
      await page.getByLabel('Repeat interval (seconds)', { exact: true }).fill('1');
      await page.getByRole('slider').focus();
      await page.keyboard.press('Home');
      const save = page.getByRole('button', { name: 'Save announcements', exact: true });
      await save.focus();
      await page.keyboard.press('Enter');
      await page.getByText('Announcement settings saved.', { exact: true }).waitFor();
      const state = await client.request('state.get', {});
      assert.equal(state.settings.announcements.speedEnabled, true);
      assert.equal(state.settings.announcements.loadEnabled, true);
      assert.equal(state.settings.announcements.volumePercent, 0);
      assert.equal(requests.filter((x) => x === 'settings.update').length, 1);
      const control = page.getByRole('group', { name: 'Lower other audio while speaking', exact: true });
      assert.equal(await control.count(), 1);
      const button = page.getByRole('button', { name: 'Test combined', exact: true });
      // Cancellation and non-primary pointers must not activate speech.
      await button.evaluate((el) =>
        el.addEventListener('pointerdown', (event) =>
          el.setAttribute('data-pointer-id', String((event as PointerEvent).pointerId))
        )
      );
      await button.hover();
      await page.mouse.down();
      const pointerId = Number(await button.getAttribute('data-pointer-id'));
      await button.dispatchEvent('pointercancel', { pointerId, isPrimary: true });
      await page.mouse.up();
      await button.click({ button: 'right' });
      assert.equal(requests.filter((x) => x === 'announcements.test').length, 0);
      await button.tap();
      await page.getByText('Test finished silently: announcement volume is muted.', { exact: true }).waitFor();
      assert.equal(requests.filter((x) => x === 'announcements.test').length, 1);
      const automatic = await browser.newPage();
      if (name === 'chromium')
        await automatic.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await automatic.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => {
          const request = JSON.parse(String(raw));
          if (request.method === 'settings.update')
            socket.send(
              JSON.stringify({ id: request.id, error: { code: 'unavailable', message: 'Test save failure' } })
            );
          else server.send(raw);
        });
        server.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          const state = message.event === 'state.changed' ? message.data : message.result;
          if (state?.announcementAudio) state.announcementAudio.loweringControl = 'automatic';
          socket.send(JSON.stringify(message));
        });
      });
      await automatic.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await automatic.goto(`${endpoint.origin}/#/settings`);
      await automatic.getByRole('slider').waitFor();
      assert.equal(
        await automatic.getByRole('group', { name: 'Lower other audio while speaking', exact: true }).count(),
        0
      );
      await automatic.getByLabel('Load threshold (%)', { exact: true }).fill('77');
      await automatic.getByRole('button', { name: 'Save announcements', exact: true }).click();
      await automatic
        .getByText('Could not save announcement settings. Your edits are kept; try again.', { exact: true })
        .waitFor();
      assert.equal(await automatic.getByLabel('Load threshold (%)', { exact: true }).inputValue(), '77');
      await automatic.close();
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
