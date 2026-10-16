import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: first launch requires durable safety agreement`, { timeout: 60_000 }, async () => {
    const endpoint = getMockBootstrap(await startMockServer(11002));
    const client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const browser = await engine.launch();
    try {
      const context = await browser.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true });
      if (name === 'chromium') await context.grantPermissions(['local-network-access'], { origin: endpoint.origin });
      await context.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      const requests: string[] = [];
      let failSave = true;
      await context.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => {
          const request = JSON.parse(String(raw));
          requests.push(request.method);
          if (request.method === 'settings.update' && failSave) {
            socket.send(JSON.stringify({ id: request.id, error: { code: 'internal', message: 'Save failed' } }));
            return;
          }
          server.send(raw);
        });
      });
      const page = await context.newPage();
      const errors: string[] = [];
      page.on('pageerror', (e) => errors.push(e.message));
      await page.goto(`${endpoint.origin}/#/settings`);
      const modal = page.getByRole('alertdialog', { name: 'Ride safely' });
      await modal.waitFor();
      await page.screenshot({ path: `/tmp/float-view-safety-${name}.png` });
      assert.equal(await page.locator('main').evaluate((node) => node instanceof HTMLElement && node.inert), true);
      assert.equal(await page.getByRole('heading', { name: 'Ride announcements' }).count(), 0);
      await page.keyboard.press('Escape');
      await page.evaluate(() => window.dispatchEvent(new Event('floatview:back', { cancelable: true })));
      await page.mouse.click(3, 3);
      assert.equal(await modal.isVisible(), true);
      assert.ok(!requests.includes('uiState.get'));
      await page.reload();
      await modal.waitFor();
      assert.equal((await client.request('state.get', {})).settings.safetyDisclaimerAccepted, false);
      const agree = modal.getByRole('button', { name: 'I agree', exact: true });
      await agree.focus();
      await page.keyboard.press('Tab');
      assert.equal(await modal.getByRole('button').count(), 1);
      assert.equal(await agree.evaluate((node) => node === document.activeElement), true);
      await page.keyboard.press('Shift+Tab');
      assert.equal(await agree.evaluate((node) => node === document.activeElement), true);
      await agree.dispatchEvent('pointerdown', { pointerId: 7, button: 2, isPrimary: true });
      await agree.dispatchEvent('pointerup', { pointerId: 7, button: 2, isPrimary: true });
      await agree.dispatchEvent('pointerdown', { pointerId: 8, button: 0, isPrimary: false });
      await agree.dispatchEvent('pointerup', { pointerId: 8, button: 0, isPrimary: false });
      await agree.hover();
      await page.mouse.down();
      await agree.dispatchEvent('pointercancel', { pointerId: 1 });
      await page.mouse.up();
      assert.equal(requests.filter((m) => m === 'settings.update').length, 0);
      await page.keyboard.press('Enter');
      await modal.getByRole('alert').waitFor();
      assert.equal((await client.request('state.get', {})).settings.safetyDisclaimerAccepted, false);
      assert.equal(await modal.isVisible(), true);
      failSave = false;
      await agree.tap();
      await modal.waitFor({ state: 'detached' });
      assert.equal((await client.request('state.get', {})).settings.safetyDisclaimerAccepted, true);
      assert.equal(requests.filter((m) => m === 'settings.update').length, 2);
      assert.ok(requests.includes('uiState.get'));
      await page.reload();
      await page.waitForFunction(
        () => document.querySelector('main')?.inert === false && document.getElementById('app-loading')?.hidden
      );
      assert.equal(await modal.count(), 0);
      assert.deepEqual(errors, []);
      await context.close();
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
