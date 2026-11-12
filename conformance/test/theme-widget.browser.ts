import assert from 'node:assert/strict';
import { readFile, mkdir } from 'node:fs/promises';
import { test } from 'node:test';
import { chromium, webkit } from 'playwright';
import { startMockServer, stopMockServer, getMockBootstrap } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const html = await readFile(new URL('../../frontend/build/index.html', import.meta.url), 'utf8');
for (const [name, engine] of Object.entries({ chromium, webkit })) {
  test(`${name}: theme widget in Live and Custom`, { timeout: 60_000 }, async () => {
    const browser = await engine.launch();
    const port = await startMockServer(0, { safetyDisclaimerAccepted: true });
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
      const ready = await client.waitForState((s) => s.activeConnection?.state === 'ready');
      const savedDeviceId = ready.activeConnection!.savedDeviceId!;
      const profile = await client.request('boardProfile.get', { savedDeviceId });
      await client.request('boardProfile.update', {
        savedDeviceId,
        expectedRevision: profile.revision,
        patch: {
          lastLiveView: 'normal',
          customDashboard: {
            version: 1,
            widgets: [{ id: 'theme', kind: 'action', action: 'themeToggle', size: { columns: 1, rows: 1 } }],
          },
        },
      });
      const page = await browser.newPage({
        viewport: { width: 390, height: 844 },
        hasTouch: true,
        reducedMotion: 'reduce',
        colorScheme: 'dark',
      });
      if (name === 'chromium')
        await page.context().grantPermissions(['local-network-access'], { origin: endpoint.origin });
      page.setDefaultTimeout(7000);
      const errors: string[] = [];
      page.on('pageerror', (error) => errors.push(error.message));
      let updates = 0;
      let fail = false;
      let delay = false;
      let release: (() => void) | undefined;
      await page.routeWebSocket('**/api/ws?*', (socket) => {
        const server = socket.connectToServer();
        socket.onMessage((raw) => {
          const message = JSON.parse(String(raw));
          if (message.method === 'settings.update' && message.params.patch.themePreference) {
            updates++;
            if (fail) {
              socket.send(
                JSON.stringify({ id: message.id, error: { code: 'invalid_request', message: 'Theme rejected' } })
              );
              return;
            }
            if (delay) {
              release = () => server.send(raw);
              return;
            }
          }
          server.send(raw);
        });
        server.onMessage((raw) => socket.send(raw));
      });
      await page.route(`${endpoint.origin}/`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
      await page.route(`${endpoint.origin}/api/bootstrap`, (route) =>
        route.fulfill({ json: { webSocketUrl: endpoint.webSocketUrl, platform: 'web' } })
      );
      await page.goto(`${endpoint.origin}/#/live`);
      const first = page.locator('.live-value-button').first();
      await first.focus();
      await page.keyboard.press('Shift+Enter');
      await page.getByRole('button', { name: 'Light / dark mode', exact: true }).click();
      await page.getByRole('dialog').waitFor({ state: 'hidden' });
      let button = page.getByRole('button', { name: 'Light / dark mode', exact: true });
      await button.waitFor();
      assert.equal(updates, 0);
      await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark');
      await button.tap();
      await page.waitForFunction(() => document.documentElement.dataset.theme === 'light');
      assert.equal(updates, 1);
      assert.equal((await client.request('state.get', {})).settings.themePreference, 'light');
      await page.reload();
      await button.waitFor();
      await page.waitForFunction(() => document.documentElement.dataset.theme === 'light');
      await button.focus();
      await page.keyboard.press('Enter');
      await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark');
      assert.equal(updates, 2);
      await button.click({ button: 'right' });
      await button.dispatchEvent('pointerdown', { pointerId: 5, isPrimary: false, button: 0 });
      await button.dispatchEvent('pointerup', { pointerId: 5, isPrimary: false, button: 0 });
      assert.equal(updates, 2);
      await page.getByRole('button', { name: 'Board actions', exact: true }).click();
      await page.getByRole('button', { name: 'Custom View', exact: true }).click();
      const custom = page.getByRole('region', { name: 'Custom dashboard', exact: true });
      await custom.waitFor();
      button = custom.getByRole('button', { name: 'Light / dark mode', exact: true });
      await button.waitFor();
      await page.waitForTimeout(350);
      const box = (await button.boundingBox())!;
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await button.dispatchEvent('pointercancel', { pointerId: 1, isPrimary: true });
      await page.mouse.up();
      assert.equal(updates, 2);
      await page.mouse.down();
      await page.waitForTimeout(700);
      await page.mouse.up();
      await page.getByRole('toolbar', { name: 'Edit dashboard' }).waitFor();
      assert.equal(updates, 2);
      await page.getByRole('button', { name: /^Configure Light \/ dark mode/ }).click();
      await page.getByLabel('Content', { exact: true }).selectOption('action:themeToggle');
      assert.equal(updates, 2);
      await page.getByRole('button', { name: 'Cancel', exact: true }).click();
      await page.getByRole('button', { name: 'Cancel', exact: true }).click();
      fail = true;
      await button.tap();
      await button.getByRole('status').filter({ hasText: 'Could not save' }).waitFor();
      assert.equal((await client.request('state.get', {})).settings.themePreference, 'dark');
      fail = false;
      delay = true;
      await button.tap();
      await button.getByRole('status').filter({ hasText: 'Saving' }).waitFor();
      assert.equal(await button.isDisabled(), true);
      release!();
      delay = false;
      await page.waitForFunction(() => document.documentElement.dataset.theme === 'light');
      assert.equal(updates, 4);
      const output = new URL(`../generated/theme-widget/${name}/`, import.meta.url);
      await mkdir(output, { recursive: true });
      await page.screenshot({ path: new URL('custom-light.png', output).pathname });
      // System preference changes update the button without a settings mutation.
      const state = await client.request('state.get', {});
      await client.request('settings.update', {
        expectedRevision: state.settings.revision,
        patch: { themePreference: 'system' },
      });
      await page.emulateMedia({ colorScheme: 'light' });
      await button.getByText('Light mode', { exact: true }).waitFor();
      await page.emulateMedia({ colorScheme: 'dark' });
      await button.getByText('Dark mode', { exact: true }).waitFor();
      await button.tap();
      await page.waitForFunction(() => document.documentElement.dataset.theme === 'light');
      assert.equal(updates, 5);
      assert.deepEqual(errors, []);
      await button.click({ button: 'right' });
      const beforeEdit = updates;
      await button.focus();
      await page.keyboard.press('Space');
      await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark');
      assert.equal(updates, beforeEdit + 1);
      const tile = custom.locator('[data-dashboard-widget]').first();
      const position = (await tile.boundingBox())!;
      await page.mouse.move(position.x + position.width / 2, position.y + position.height / 2);
      await page.mouse.down();
      await page.waitForTimeout(700);
      await page.mouse.up();
      await custom.getByRole('button', { name: 'Add widget', exact: true }).click();
      await page.getByRole('searchbox', { name: 'Search widgets', exact: true }).fill('theme');
      await page
        .getByRole('dialog', { name: 'Add widget', exact: true })
        .getByRole('button', { name: 'Light / dark mode', exact: true })
        .click();
      await page.getByRole('button', { name: '2 columns by 1 row', exact: true }).click();
      await page.getByRole('button', { name: 'Add widget', exact: true }).click();
      await page
        .getByRole('toolbar', { name: 'Edit dashboard' })
        .getByRole('button', { name: 'Save dashboard', exact: true })
        .click();
      assert.equal(updates, beforeEdit + 1, 'preview and editor never toggle appearance');
      const saved = (await client.request('boardProfile.get', { savedDeviceId })).values.customDashboard!;
      assert.equal(saved.widgets.length, 2);
      assert.deepEqual(saved.widgets[1]!.size, { columns: 2, rows: 1 });
      await page.reload();
      await custom.locator('[data-dashboard-widget]').nth(1).waitFor();
      assert.equal(
        (await client.request('boardProfile.get', { savedDeviceId })).values.customDashboard!.widgets[0]!.kind,
        'action'
      );
    } finally {
      await browser.close();
      client.close();
      await stopMockServer();
    }
  });
}
